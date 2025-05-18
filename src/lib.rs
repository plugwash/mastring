//! MAString, a string type designed to minimise memory allocations.
//!
//! This crate provides four types, MAByteString stores arbitrary
//! sequences of bytes, while MAString stores valid UTF-8.
//! MAByteStringBuilder and MAStringbuilder are similar to 
//! MAByteString and MAString but do not allow shared ownership.
//!
//! There are a number of reference-counted string types for rust,
//! However, these commonly require memory allocation when converting
//! From a std::String, which means adopting them can actually
//! increase memory allocator calls.
//!
//! This crate attempts to solve that, it is currently at the alpha
//! stage and has only been very lightly tested.
//!
//! A MAString or MABytestring is four pointers in size, and can be in one of
//! five modes, the mode can be checked through the "mode" method.
//! which returns a string representing the current mode and if the string
//! is in a shared ownership mode whether or not it is actually shared.
//! 
//! There are five possible modes.
//! * Short string ("short"): the string data is stored entirely
//!   within the MAString object.
//! * Static string ("static"): the string stores a pointer to a
//!   string with static lifetime
//! * Uniquely owned string ("unique"): the string stores a pointer to a
//!   string that is uniquely owned by the current MAString object.
//! * Reference counted string with inline control block  ("cbinline"):
//!   the string stores a pointer to a reference counted string,
//! * Reference counted string with seperate control block  ("cbowned"):
//!   the string stores a pointer to a reference counted string,
//!
//! A uniquely owned MAString will be converted to one with shared
//! ownership, and a seperate control block by Clone calls. This
//! means repeatedly cloning a string will result in at-most
//! a single memory allocation.
//!
//! MAStrings are represented as a union. The short string variant
//! stores string data and a single byte length. 0x80 is added to
//! the length of a short string to distinguish it from a long 
//! string.
//!
//! The long string variant stores a pointer, length, "capacity"
//! and control block pointer, the "capacity" includes space
//! used to store an inline control block if-any and is set
//! to zero to indicate a static string.
//!
//! The most significant byte of the long string length shares
//! A memory location with the short string length. Since the
//! long length field is always less than isize::max and the 
//! short length field is always greater than or equal to 0x80,
//! this allows short and long strings to be distinguished.  The control
//! block pointer is stored as an atomic pointer, to allow a
//! uniquely owned string to be converted to a shared ownership
//! string by the clone function.
//!
//! The control block is an atomic usize, with the lower
//! bit used to distinguish between seperately owned, and inline
//! control blocks, and the remaining bits used as a reference
//! count.

//#![no_std]
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::sync::atomic::fence;
use core::mem::size_of;
use core::mem::ManuallyDrop;
use core::mem;
use core::mem::align_of;
use core::ptr;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::Add;
use core::ops::AddAssign;
use core::slice;
use core::cmp::max;
use core::cmp::min;

extern crate alloc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::str;
use alloc::string::String;
use alloc::string::FromUtf8Error;
use alloc::fmt;

//contol block values
//stores reference count * 2
//even numbers for owned
//odd numbers for inline

#[repr(C)]
struct InnerLong {
    #[cfg(target_endian="big")]
    len: usize,
    cap: usize,
    ptr: * mut u8,
    cbptr: AtomicPtr<AtomicUsize>,
    #[cfg(target_endian="little")]
    len: usize,
}

impl InnerLong {
    #[inline]
    fn from_vec(mut v: Vec<u8>, allowcb: bool, mincap: usize) -> Self {
        // it would be nice to use into_raw_parts here, but it's unstable.
        let len = v.len();
        let cap = v.capacity();
        assert!(mincap <= cap);
        let mincap = max(len,mincap);
        let ptr = v.as_mut_ptr();
        mem::forget(v);
        let mut cbptr : * mut AtomicUsize = ptr::null_mut();
        if allowcb {
            //println!("checking if we have room for a control block");
            unsafe {
                //check if we have room for a control block.
                //math wont overflow because a vec is limited to isize,
                //which has half the range of usize.
                let end = ptr.add(mincap);
                let mut cbstart = mincap + end.align_offset(align_of::<AtomicUsize>());
                let cbrequired = cbstart + size_of::<AtomicUsize>();
                if cbrequired <= cap {
                    let cbextraspace = (cap - cbrequired) & !(align_of::<AtomicUsize>()-1);
                    cbstart += cbextraspace;
                    //println!("allocating control block");
                    cbptr = ptr.add(cbstart) as * mut AtomicUsize;
                    *cbptr = AtomicUsize::new(3);
                } else {
                    //println!("no room for control block!");
                }
            }
        }
        InnerLong { len : len, cap: cap, ptr: ptr, cbptr: AtomicPtr::new(cbptr) }
    }

    #[inline]
    fn from_slice(s: &[u8], allowcb: bool, mincap: usize) -> Self {
        let len = max(s.len(),mincap);
        let mask = align_of::<AtomicUsize>() - 1;
        let veccap = ((len + mask) & !mask) + size_of::<AtomicUsize>();
        //println!("len:{len} allowcb:{allowcb} veccap:{veccap}");
        let mut v = Vec::with_capacity(veccap);
        v.extend_from_slice(s);
        Self::from_vec(v,allowcb,mincap)
    }

    // ensure the pointer is unique
    // if the string is copied, then mincap sets the minimum capacity of the new
    // string, excluding control block space. However if the string is not copied
    // then it's capacity is left unchanged.
    #[inline]
    fn make_unique(&mut self, mincap: usize, allowcb: bool) {
        //println!("in make_unique mincap={mincap} allowcb={allowcb}");
        if self.cap == 0 { // static string, we need to copy
            unsafe {
                *self = InnerLong::from_slice(slice::from_raw_parts(self.ptr,self.len),allowcb,mincap);
            }
        } else {
            let cbptr = self.cbptr.load(Ordering::Relaxed);
            if cbptr.is_null() {
                // we already have unique ownership of the string.
            } else {
                unsafe {
                    let refcount = (*cbptr).load(Ordering::Relaxed) >> 1;
                    if refcount == 1 {
                        // we are the only owner of the String
                        if !allowcb {
                            // switch it from shared ownership mode to unique ownership mode.
                            if ((*cbptr).load(Ordering::Relaxed) & 1) == 0 {
                                //free control block pointer
                                let _ = Box::from_raw(cbptr);
                            }
                            self.cbptr.store(ptr::null_mut(),Ordering::Relaxed);
                        }
                    } else {
                        // there are other owners, we need to copy
                        *self = InnerLong::from_slice(slice::from_raw_parts(self.ptr,self.len),allowcb,mincap) ;
                    }
                }
            }
        }
    }

    fn usablecap(&self) -> usize {
        // check for an inline control block.
        let ptr = self.ptr as usize;
        // use relaxed as the only time the value critically matters is
        // when we already have unique ownership.
        let cbptr = self.cbptr.load(Ordering::Relaxed) as usize;
        if cbptr < ptr {
            // no control block, or outline control block located before data
            self.cap
        } else {
            // inline control block, or outline control block located after data.
            min(cbptr - ptr,self.cap)
        }
    }

    // reallocate the buffer if it's capacity is less than requested.
    // implement an exponential reallocation
    // SAFETY: callers must ensure that the innerlong has unique ownership
    // before calling, use make_unique if needed.
    unsafe fn reserve(&mut self, mut mincap: usize, allowcb: bool) {
        let cap = self.cap;
        if mincap <= cap { 
            if mincap <= self.usablecap() { return }
            // an inline control block is reducing our usable capacity, get rid of it
            self.cbptr = AtomicPtr::new(ptr::null_mut());
            return
        };
        mincap = max(mincap, cap * 2);
        *self = Self::from_slice(slice::from_raw_parts(self.ptr, self.len), allowcb,  mincap);
    }

}

impl Drop for InnerLong {
    fn drop(&mut self) {
        let len = self.len;
        let cap = self.cap;
        if cap == 0 { return } //static string
        let cbptr = self.cbptr.load(Ordering::Relaxed);
        unsafe {
            if !cbptr.is_null() {
                let oldcb = (*cbptr).fetch_sub(2, Ordering::Release); //decrease the refcount
                if oldcb > 3 { return } // there are still other references
                fence(Ordering::Acquire);
                if (oldcb & 1) == 0 { //owned control block
                    let _ = Box::from_raw(cbptr);
                }
            }
            // we hold the only reference, turn it back into a vec so rust will free it.
            let _ = Vec::from_raw_parts(self.ptr, len, cap);
        }
    }

}

const SHORTLEN : usize = size_of::<InnerLong>()-1;

#[repr(C)]
#[derive(Clone,Copy)]
struct InnerShort {
    #[cfg(target_endian="big")]
    len: u8,
    data: [u8;SHORTLEN],
    #[cfg(target_endian="little")]
    len: u8,
}

///
#[repr(C)]
pub union MAByteString {
    short: InnerShort,
    long: ManuallyDrop<InnerLong>,
}
unsafe impl Send for MAByteString {}
unsafe impl Sync for MAByteString {}


impl MAByteString {
    /// Creates a new MAByteString.
    /// This will not allocate
    pub const fn new() -> Self {
        MAByteString { short: InnerShort { data: [0; SHORTLEN] , len: 0x80 } }
    }

    /// Creates a MAByteString from a slice.
    /// This will allocate if the string cannot be stored as a short string,
    /// the resulting string will be in shared ownership mode with an inline
    /// control block, so cloning will not result in further allocations.
    pub fn from_slice(s: &[u8]) -> Self {
        let len = s.len();
        if len <= SHORTLEN {
            let mut data : [u8; SHORTLEN] = [0; SHORTLEN];
            data[0..len].copy_from_slice(&s);
            MAByteString { short: InnerShort { data: data, len: len as u8 + 0x80 } }
        } else {
            MAByteString { long: ManuallyDrop::new(InnerLong::from_slice(s, true,0)) }
        }
    }


    /// create a MAByteString from a Vec.
    /// This will not allocate.
    /// If the string can be represented as a  short string then it will be stored
    /// as one and the memory owned by the
    /// Vec will be freed. Otherwise if the vec has sufficient free storage
    /// to store an inline control block, then the memory owned by the vec will
    /// be used to createa shared ownership MAString with an inline control block.
    /// if neither of those are possible, then the MAString will have unique
    /// ownership, until it is first Cloned, at which point it will switch to
    /// shared ownership with an external control block. 
    pub fn from_vec(v: Vec<u8>) -> Self {
        let len = v.len();
        if len <= SHORTLEN {
            let mut data : [u8; SHORTLEN] = [0; SHORTLEN];
            data[0..len].copy_from_slice(&v);
            MAByteString { short: InnerShort { data: data, len: len as u8 + 0x80 } }
        } else {
            MAByteString { long: ManuallyDrop::new(InnerLong::from_vec(v, true, 0)) }
        }
    }

    /// Create a MAByteString from a static reference
    /// This function will not allocate, and neither wil
    /// Clones of the MAString thus created.
    pub const fn from_static(s: &'static [u8]) -> Self {
        let len = s.len();
        if len <= SHORTLEN {
            let mut data : [u8; SHORTLEN] = [0; SHORTLEN];
            let mut i = 0;
            while i < len {
                 data[i] = s[i];
                 i += 1;
            }
            //data[0..len].copy_from_slice(&s); // doesn't work in const fn
            MAByteString { short: InnerShort { data: data, len: len as u8 + 0x80 } }
        } else {
            MAByteString { long: ManuallyDrop::new(InnerLong { len : len, cap: 0, ptr: s.as_ptr() as *mut u8, cbptr: AtomicPtr::new(ptr::null_mut()) }) }
        }
    }

    pub fn from_builder(b : MAByteStringBuilder) -> Self {
        unsafe {
            let len = b.long.len;
            if len > isize::max as usize {
                return MAByteString { short: b.short };
            }
            let cap = b.long.cap;
            let ptr = b.long.ptr;
            mem::forget(b);
            //check if we have room for a control block.
            //math wont overflow because a vec is limited to isize,
            //which has half the range of usize.
            let end = ptr.add(len);
            let cbstart = len + end.align_offset(align_of::<AtomicUsize>());
            let cbrequired = cbstart + size_of::<AtomicUsize>();
            let mut cbptr : * mut AtomicUsize = ptr::null_mut();
            if cbrequired <= cap {
                cbptr = ptr.add(cbstart) as * mut AtomicUsize;
                *cbptr = AtomicUsize::new(3);
            }
            MAByteString { long: ManuallyDrop::new(InnerLong { len : len, cap: cap, ptr: ptr, cbptr: AtomicPtr::new(cbptr) }) }
        }
    }

    /// Return the current mode of the MAByteString (for testing/debugging)
    /// The strings returned from this function are not considred stable, and
    /// changes to them are not considered a semver break.
    pub fn get_mode(&self) -> &'static str {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                "short"
            } else if self.long.cap == 0 { // static string
                "static"
            } else {
                let cbptr = self.long.cbptr.load(Ordering::Acquire);
                if cbptr.is_null() {
                    "unique"
                } else { 
                    let cbval = (*cbptr).load(Ordering::Relaxed);
                    //println!("cbval = {}",cbval);
                    if (cbval & 1) == 0 {
                        if cbval <= 3 {
                            "cbowned (unique)"
                        } else {
                            "cbowned (shared)"
                        }
                    } else {
                        if cbval <= 3 {
                            "cbinline (unique)"
                        } else {
                            "cbinline (shared)"
                        }
                    }
                }
            }
        }

    }

    /// ensure there is capacity for at least extracap more bytes
    /// beyond the current length of the string
    /// and return the pointer and len, and the flag that indicates
    /// whether we are in short mode this saves duplicate
    /// work in the functions that need to reserve space and
    /// then use it.
    fn reserve_extra_internal(&mut self, extracap: usize) -> (*mut u8, usize, bool) {
        unsafe {
            let mut len = self.long.len;
            let mincap;
            //println!("entering reserve_extra_internal mode={} capacity={}",self.get_mode(),self.capacity());
            if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                mincap = len + extracap;
                if mincap > SHORTLEN {
                    let mincap = max(mincap,SHORTLEN*2);
                    *self = Self { long: ManuallyDrop::new(InnerLong::from_slice(slice::from_raw_parts(self.short.data.as_ptr(),len),true,mincap)) }
                } else {
                    //println!("returning from reserve_extra_internal mode={} capacity={}",self.get_mode(),self.capacity());
                    return (self.short.data.as_mut_ptr(),len, true);
                }
            } else {
                mincap = len + extracap;
                self.long.deref_mut().make_unique(mincap,true);
                //println!("called make_unique mode={} capacity={}",self.get_mode(),self.capacity());
                self.long.deref_mut().reserve(mincap,true);
                //println!("returning from reserve_extra_internal mode={} capacity={}",self.get_mode(),self.capacity());
            }
            // if we reach here, we know it's a "long" String.
            return (self.long.ptr, len, false);
        }
    }

    /// ensure there is capacity for at least mincap bytes
    pub fn reserve(&mut self, mincap: usize) {
        unsafe {
            let mut len = self.long.len;
            if len > isize::max as usize {  //inline string
                if mincap > SHORTLEN {
                    len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                    let mincap = max(mincap,SHORTLEN*2);
                    *self = Self { long: ManuallyDrop::new(InnerLong::from_slice(slice::from_raw_parts(self.short.data.as_ptr(),len),true,mincap)) }
                }
            } else {
                self.long.deref_mut().make_unique(mincap,true);
                self.long.deref_mut().reserve(mincap,true);
            }
        }
    }

    /// report the "capacity" of the string. Be aware that due to MAByteStrings
    /// copy on write model, this does not gaurantee that future operations
    /// will not allocate. Returns zero for static strings.
    pub fn capacity(&self) -> usize {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                //println!("short string");
                SHORTLEN
            } else {
                //println!("long string, self.long.cap = {}",self.long.cap);
                self.long.usablecap()
            }
        }
    }

    // clears the string.
    // if the string is in unique ownership mode, or is in shared ownership
    // mode but we are the only owner then this will reset the length but
    // leave the mode and capacity untouched. Otherwise the string will be
    // reset to an empty short string.
    pub fn clear(&mut self) {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                self.short.len = 0x80;
            } if self.long.cap == 0 { // static string
                self.short.len = 0x80;
            } else {
                let cbptr = self.long.cbptr.load(Ordering::Relaxed);
                if cbptr.is_null() { // unique ownership mode.
                    self.long.len = 0;
                } else {
                    let refcount = (*cbptr).load(Ordering::Relaxed) >> 1;
                    if refcount == 1 {
                        // we are the only owner of the String
                        self.long.len = 0;
                    } else {
                        // there are other owners, we need to seperate ourselves from them.
                        *self = Self::new();
                    }
                }
            }
        }
    }

    /// convert the MAByteStirng into a Vec, this may allocate.
    pub fn into_vec(mut self) -> Vec<u8> {
        unsafe {
            let mut len = self.long.len;
            if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                return slice::from_raw_parts(self.short.data.as_ptr(), len).to_vec();
            }
            self.long.deref_mut().make_unique(0,true);
            let cap = self.long.cap;
            let ptr = self.long.ptr;
            mem::forget(self);
            return Vec::from_raw_parts(ptr,len,cap);
        }
    }
}

impl Drop for MAByteString {
    fn drop(&mut self) {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize { return }; //inline string
            ManuallyDrop::drop(&mut self.long);
        }
    }
}

impl Clone for MAByteString{
    fn clone(&self) -> Self {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                MAByteString { short : self.short }
            } else if self.long.cap == 0 { // static string
                MAByteString { long : ManuallyDrop::new(InnerLong { len : len, cap: 0, ptr: self.long.ptr, cbptr: AtomicPtr::new(ptr::null_mut()) }) }
            } else {
                let mut cbptr = self.long.cbptr.load(Ordering::Acquire);
                if cbptr.is_null() {
                    let newcbptr = Box::into_raw(Box::new(AtomicUsize::new(2)));
                    if let Err(cxcbptr) = self.long.cbptr.compare_exchange(ptr::null_mut(),newcbptr,Ordering::AcqRel,Ordering::Acquire) {
                        let _ = Box::from_raw(newcbptr);
                        cbptr = cxcbptr;
                    } else  {
                        cbptr = newcbptr
                    }
                }
                if (*cbptr).fetch_add(2, Ordering::Relaxed) > usize::MAX / 2 {
                    (*cbptr).fetch_sub(2, Ordering::Relaxed);
                    panic!("reference count too high, you have a refrence leak");
                }
                MAByteString { long : ManuallyDrop::new(InnerLong { len : len, cap: self.long.cap, ptr: self.long.ptr, cbptr: AtomicPtr::new(cbptr) }) }
            }
        }
    }
}

impl Deref for MAByteString {
   type Target = [u8];
   #[inline]
   fn deref(&self) -> &[u8] {
        unsafe {
            let mut len = self.long.len;
            let ptr = if len > isize::max as usize {
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                self.short.data.as_ptr()
            } else {
                self.long.ptr
            };
            slice::from_raw_parts(ptr,len)
        }
   }
}


impl DerefMut for MAByteString {
   #[inline]
   fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            let mut len = self.long.len;
            let ptr = if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                self.short.data.as_mut_ptr()
            } else { 
                self.long.deref_mut().make_unique(0,true);
                // if we get here we have unique owenership of the data
                // either by being in unique mode, or by being in shared
                // ownership mode but being the only owner.
                self.long.ptr
            };
            slice::from_raw_parts_mut(ptr,len)
        }
   }
}

impl Add<&[u8]> for MAByteString {
    type Output = Self;
    fn add(mut self, rhs: &[u8]) -> Self {
        self += rhs;
        self
    }
}

impl AddAssign<&[u8]> for MAByteString {
    fn add_assign(&mut self, other: &[u8]) {
        unsafe {
            let (ptr, mut len, short) = self.reserve_extra_internal(other.len());
            ptr::copy_nonoverlapping(other.as_ptr(), ptr.add(len), other.len());
            len += other.len();
            if short {
                self.short.len = (len + 0x80) as u8;
            } else {
                self.long.len = len;
            }
        }
    }
}

fn bytes_debug(s: &[u8], f: &mut fmt::Formatter<'_>) -> Result<(),fmt::Error> {
    f.write_str("b\"")?;
    let mut groupstart = 0;
    let mut p = 0;
    let len = s.len();
    while p < len {
        let c = s[p];
        if (c < 0x20) || (c > 0x7E) || (c == b'\\') || (c == b'\"') {
            // we found a character that can't be written directly
            // check if there are any characters waiting to be written
            // before the current one
            if groupstart < p {
                unsafe {
                    // safety: we have validated that this subsequence is ascii only
                    f.write_str(str::from_utf8_unchecked(&s[groupstart..p]))?;
                }
            }
            // write an escape for the current char
            if c == b'\\' {
                f.write_str("\\\\")?;
            } else if c == b'\"' {
                f.write_str("\\\"")?;
            } else {
                let mut escaped: [u8;4] = *b"\\xxx";
                let mut uppernibble = c >> 4;
                if uppernibble < 10 {
                    uppernibble += b'9'
                } else {
                    uppernibble -= 10;
                    uppernibble += b'a';
                }
                escaped[2] = uppernibble;
                let mut lowernibble = c & 0xF;
                if lowernibble < 10 {
                    lowernibble += b'9'
                } else {
                    lowernibble -= 10;
                    lowernibble += b'a';
                }
                escaped[3] = lowernibble;
                unsafe {
                    //safety: we know the string is ascii.
                    f.write_str(str::from_utf8_unchecked(&escaped))?;
                }
                groupstart = p + 1;
            }
        }
        p += 1;
    }
    if groupstart < len {
        unsafe {
            // safety: we have validated that this subsequence is ascii only
            f.write_str(str::from_utf8_unchecked(&s[groupstart..len]))?;
        }
    }
    f.write_str("\"")?;
    Ok(())

}

impl fmt::Debug for MAByteString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(),fmt::Error> {
        bytes_debug(self,f)
    }
}

impl PartialEq for MAByteString {
    fn eq(&self, other : &MAByteString) -> bool {
         return self.deref() == other.deref();
    }
}
impl Eq for MAByteString {}

impl PartialEq<&[u8]> for MAByteString {
    fn eq(&self, other : &&[u8]) -> bool {
         return self.deref() == *other;
    }
}

impl PartialEq<MAByteString> for &[u8] {
    fn eq(&self, other : &MAByteString) -> bool {
         return *self == other.deref();
    }
}

impl<const N: usize> PartialEq<&[u8;N]> for MAByteString {
    fn eq(&self, other : &&[u8;N]) -> bool {
         return self.deref() == *other;
    }
}

impl<const N: usize>  PartialEq<MAByteString> for &[u8;N] {
    fn eq(&self, other : &MAByteString) -> bool {
         return *self == other.deref();
    }
}

#[repr(C)]
pub union MAByteStringBuilder {
    short: InnerShort,
    long: ManuallyDrop<InnerLong>,
}
unsafe impl Send for MAByteStringBuilder {}
unsafe impl Sync for MAByteStringBuilder {}


impl MAByteStringBuilder {
    /// Creates a new MAByteStringBuilder.
    /// This will not allocate
    pub const fn new() -> Self {
        MAByteStringBuilder { short: InnerShort { data: [0; SHORTLEN] , len: 0x80 } }
    }

    /// Creates a MAByteStringBuilder from a slice.
    /// This will allocate if the string cannot be stored as a short string,
    pub fn from_slice(s: &[u8]) -> Self {
        let len = s.len();
        if len <= SHORTLEN {
            let mut data : [u8; SHORTLEN] = [0; SHORTLEN];
            data[0..len].copy_from_slice(&s);
            MAByteStringBuilder { short: InnerShort { data: data, len: len as u8 + 0x80 } }
        } else {
            MAByteStringBuilder { long : ManuallyDrop::new(InnerLong::from_slice(s,false,0)) }
        }
    }

    /// create a MAByteStringBuilder from a Vec.
    /// This will not allocate.
    /// If the string can be represented as a  short string then it will be stored
    /// as one and the memory owned by the Vec will be freed.
    pub fn from_vec(v: Vec<u8>) -> Self {
        let len = v.len();
        if len <= SHORTLEN {
            let mut data : [u8; SHORTLEN] = [0; SHORTLEN];
            data[0..len].copy_from_slice(&v);
            MAByteStringBuilder { short: InnerShort { data: data, len: len as u8 + 0x80 } }
        } else {
            MAByteStringBuilder { long: ManuallyDrop::new(InnerLong::from_vec(v,false,0)) }
        }
    }

    pub fn from_mabs(mut s: MAByteString) -> Self {
        unsafe {
            let len = s.long.len;
            if len > isize::max as usize {  //inline string
                MAByteStringBuilder { short: s.short }
            } else {
                s.long.deref_mut().make_unique(0,false);
                let inner = ptr::read(&mut s.long);
                mem::forget(s);
                MAByteStringBuilder { long: inner }
            }
        }
    }

    /// Return the current mode of the MAByteStringBuilder (for testing/debugging)
    /// This includes logic to detect states that are valid for MAByteString, but
    /// not for MAByteStringBuilder.
    pub fn get_mode(&self) -> &'static str {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                "short"
            } else if self.long.cap == 0 { // static string
                "static (invalid)"
            } else {
                let cbptr = self.long.cbptr.load(Ordering::Acquire);
                if cbptr.is_null() {
                    "unique"
                } else if ((*cbptr).load(Ordering::Relaxed) & 1) == 0 {
                    "cbowned (inavlid)"
                } else {
                    "cbinline (invalid)"
                }
            }
        }
    }

    /// ensure there is capacity for at least extracap more bytes
    /// beyond the current length of the string
    /// and return the pointer and len, and the flag that indicates
    /// whether we are in short mode this saves duplicate
    /// work in the functions that need to reserve space and
    /// then use it.
    fn reserve_extra_internal(&mut self, extracap: usize) -> (*mut u8, usize, bool) {
        unsafe {
            let mut len = self.long.len;
            let mincap;
            if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                mincap = len + extracap;
                if mincap > SHORTLEN {
                    let mincap = max(mincap,SHORTLEN*2);
                    *self = Self { long: ManuallyDrop::new(InnerLong::from_slice(slice::from_raw_parts(self.short.data.as_ptr(),len),false,mincap)) }
                } else {
                    return (self.short.data.as_mut_ptr(),len, true);
                }
            } else {
                mincap = len + extracap;
                self.long.deref_mut().reserve(mincap,false);
            }
            // if we reach here, we know it's a "long" String.
            return (self.long.ptr, len, false);
        }
    }

    /// ensure there is capacity for at least mincap bytes
    pub fn reserve(&mut self, mincap: usize) {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                if mincap > SHORTLEN {
                    let mincap = max(mincap,SHORTLEN*2);
                    *self = Self { long: ManuallyDrop::new(InnerLong::from_slice(slice::from_raw_parts(self.short.data.as_ptr(),len),false,mincap)) }
                }
            } else {
                self.long.deref_mut().reserve(mincap,false);
            }
        }
    }

    /// report the "capacity" of the string.
    pub fn capacity(&self) -> usize {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                SHORTLEN
            } else {
                self.long.cap
            }
        }
    }

    // clears the string, retaining it's capacity.
    pub fn clear(&mut self) {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                self.short.len = 0x80;
            } else {
                self.long.len = 0;
            }
        }
    }

    /// convert the MAByteStringBuilder into a Vec, this may allocate.
    pub fn into_vec(self) -> Vec<u8> {
        unsafe {
            let mut len = self.long.len;
            if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                return slice::from_raw_parts(self.short.data.as_ptr(), len).to_vec();
            }
            let cap = self.long.cap;
            let ptr = self.long.ptr;
            mem::forget(self);
            return Vec::from_raw_parts(ptr,len,cap);
        }
    }

    

}

impl Drop for MAByteStringBuilder {
    fn drop(&mut self) {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize { return }; //inline string
            let cap = self.long.cap;
            // we hold the only reference, turn it back into a vec so rust will free it.
            let _ = Vec::from_raw_parts(self.long.ptr, self.long.len, cap);
        }
    }
}

impl Clone for MAByteStringBuilder {
    fn clone(&self) -> Self {
        MAByteStringBuilder::from_slice(self)
    }
}

impl Deref for MAByteStringBuilder {
   type Target = [u8];
   #[inline]
   fn deref(&self) -> &[u8] {
        unsafe {
            let mut len = self.long.len;
            let ptr = if len > isize::max as usize {
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                self.short.data.as_ptr()
            } else {
                self.long.ptr
            };
            slice::from_raw_parts(ptr,len)
        }
   }
}

impl DerefMut for MAByteStringBuilder {
   #[inline]
   fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            let mut len = self.long.len;
            let ptr = if len > isize::max as usize {
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                self.short.data.as_mut_ptr()
            } else {
                self.long.ptr
            };
            slice::from_raw_parts_mut(ptr,len)
        }
   }
}

impl Add<&[u8]> for MAByteStringBuilder {
    type Output = Self;
    fn add(mut self, rhs: &[u8]) -> Self {
        self += rhs;
        self
    }
}

impl AddAssign<&[u8]> for MAByteStringBuilder {
    fn add_assign(&mut self, other: &[u8]) {
        unsafe {
            let (ptr, mut len, short) = self.reserve_extra_internal(other.len());
            ptr::copy_nonoverlapping(other.as_ptr(), ptr.add(len), other.len());
            len += other.len();
            if short {
                self.short.len = (len + 0x80) as u8;
            } else {
                self.long.len = len;
            }
        }
    }
}

impl fmt::Debug for MAByteStringBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(),fmt::Error> {
        bytes_debug(self,f)
    }
}

impl PartialEq for MAByteStringBuilder {
    fn eq(&self, other : &MAByteStringBuilder) -> bool {
         return self.deref() == other.deref();
    }
}
impl Eq for MAByteStringBuilder {}

impl PartialEq<&[u8]> for MAByteStringBuilder {
    fn eq(&self, other : &&[u8]) -> bool {
         return self.deref() == *other;
    }
}

impl PartialEq<MAByteStringBuilder> for &[u8] {
    fn eq(&self, other : &MAByteStringBuilder) -> bool {
         return *self == other.deref();
    }
}

impl<const N: usize> PartialEq<&[u8;N]> for MAByteStringBuilder {
    fn eq(&self, other : &&[u8;N]) -> bool {
         return self.deref() == *other;
    }
}

impl<const N: usize>  PartialEq<MAByteStringBuilder> for &[u8;N] {
    fn eq(&self, other : &MAByteStringBuilder) -> bool {
         return *self == other.deref();
    }
}

#[derive(Clone)]
pub struct MAString {
    inner: MAByteString,
}

impl MAString {
    /// Creates a new MAString.
    pub const fn new() -> Self {
        MAString { inner: MAByteString::new() }
    }

    /// Creates a MAString from a slice.
    /// This will allocate if the string cannot be stored as a short string,
    /// the resulting string will be in shared ownership mode with an inline
    /// control block, so cloning will not result in further allocations.
    pub fn from_slice(s: &str) -> Self {
        MAString { inner: MAByteString::from_slice(s.as_bytes()) }
    }

    /// create a MAString from a std::String.
    /// This will not allocate.
    /// If the string can be represented as a  short string then it will be stored
    /// as one and the memory owned by the
    /// Vec will be freed. Otherwise if the vec has sufficient free storage
    /// to store an inline control block, then the memory owned by the vec will
    /// be used to createa shared ownership MAString with an inline control block.
    /// if neither of those are possible, then the MAString will have unique
    /// ownership, until it is first Cloned, at which point it will switch to
    /// shared ownership with an external control block. 
    pub fn from_string(s: String) -> Self {
        MAString { inner: MAByteString::from_vec(s.into_bytes()) }
    }

    /// Create a MAByteString from a static reference
    /// This function will not allocate, and neither wil
    /// Clones of the MAString thus created.
    pub const fn from_static(s: &'static str) -> Self {
        MAString { inner: MAByteString::from_static(s.as_bytes()) }
    }

    /// Return the current mode of the MAByteString (for testing/debugging)
    pub fn get_mode(&self) -> &'static str {
        self.inner.get_mode()
    }

    /// Converts to a mutable string slice.
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe {
            str::from_utf8_unchecked_mut(&mut self.inner)
        }
    }

    /// ensure there is capacity for at least mincap bytes
    pub fn reserve(&mut self, mincap: usize) {
        self.inner.reserve(mincap);
    }

    /// report the "capacity" of the string. Be aware that due to MAByteStrings
    /// copy on write model, this does not gaurantee that future operations
    /// will not allocate.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    // clears the string.
    // if the string is in unique ownership mode, or is in shared ownership
    // mode but we are the only owner then this will reset the length but/
    // leave the capacity untouched. Otherwise the string will be reset to
    // an empty short string.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// convert the MAString into a Vec, this may allocate.
    pub fn into_vec(self) -> Vec<u8> {
        self.inner.into_vec()
    }

    /// fills the string with UTF-8 data. SAFETY: It is UB to supply invalid UTF-8
    pub unsafe fn from_utf8_unchecked(data: MAByteString) -> Self {
        Self { inner: data }
    }

    /// fills the string with UTF-8 data, returning an error if it is invalid
    pub fn from_utf8(data: MAByteString) -> Result<Self, FromUtf8Error> {
        match str::from_utf8(&data) {
            Ok(..) => Ok( Self { inner: data } ),
            Err(..) => String::from_utf8(data.into_vec()).map(|_| unreachable!()),
        }
    }

    /// fills the string with UTF-8 data, returning an error if it is invalid
    pub fn from_utf8_lossy(data: MAByteString) -> Self {
        match str::from_utf8(&data) {
            Ok(..) => Self { inner: data },
            Err(..) => Self::from_string(String::from_utf8_lossy(&data).into_owned()),
        }
    }

    /// convert the MAString into a Std::string, this may allocate.
    pub fn into_string(self) -> String {
        unsafe {
            String::from_utf8_unchecked(self.inner.into_vec())
        }
    }

    // convert the MAString into a MAByteString
    pub fn into_bytes(self) -> MAByteString {
        self.inner
    }
}

impl Deref for MAString {
   type Target = str;
   #[inline]
   fn deref(&self) -> &str {
        unsafe {
            str::from_utf8_unchecked(&self.inner)
        }
   }
}

impl DerefMut for MAString {
   #[inline]
   fn deref_mut(&mut self) -> &mut str {
        unsafe {
            str::from_utf8_unchecked_mut(&mut self.inner)
        }
   }
}

impl fmt::Display for MAString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl fmt::Debug for MAString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(),fmt::Error> {
        fmt::Debug::fmt(self.deref(),f)
    }
}

impl PartialEq for MAString {
    fn eq(&self, other : &MAString) -> bool {
         return self.deref() == other.deref();
    }
}
impl Eq for MAString {}

impl PartialEq<&str> for MAString {
    fn eq(&self, other : &&str) -> bool {
         return self.deref() == *other;
    }
}

impl PartialEq<MAString> for &str {
    fn eq(&self, other : &MAString) -> bool {
         return *self == other.deref();
    }
}

#[derive(Clone)]
pub struct MAStringBuilder {
    inner: MAByteStringBuilder,
}

impl MAStringBuilder {
    /// Creates a new MAString.
    pub const fn new() -> Self {
        MAStringBuilder { inner: MAByteStringBuilder::new() }
    }

    /// Creates a MAStringBuilder from a slice.
    /// This will allocate if the StringBuilder cannot be stored as a short string,
    /// the resulting string will be in shared ownership mode with an inline
    /// control block, so cloning will not result in further allocations.
    pub fn from_slice(s: &str) -> Self {
        MAStringBuilder { inner: MAByteStringBuilder::from_slice(s.as_bytes()) }
    }

    /// create a MAStringBuilder from a std::String.
    /// This will not allocate.
    /// If the string can be represented as a  short string then it will be stored
    /// as one and the memory owned by the Vec will be freed.
    pub fn from_string(s: String) -> Self {
        MAStringBuilder { inner: MAByteStringBuilder::from_vec(s.into_bytes()) }
    }

    /// Return the current mode of the MAByteStringBuilder (for testing/debugging)
    pub fn get_mode(&self) -> &'static str {
        self.inner.get_mode()
    }

    /// Converts to a mutable string slice.
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe {
            str::from_utf8_unchecked_mut(&mut self.inner)
        }
    }

    /// ensure there is capacity for at least mincap bytes
    pub fn reserve(&mut self, mincap: usize) {
        self.inner.reserve(mincap);
    }

    /// report the "capacity" of the string. Be aware that due to MAByteStrings
    /// copy on write model, this does not gaurantee that future operations
    /// will not allocate.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    // clears the string, retaining it's capacity.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// convert the MAStringBuilder into a Vec, this may allocate.
    pub fn into_vec(self) -> Vec<u8> {
        self.inner.into_vec()
    }

    /// fills the string with UTF-8 data. SAFETY: It is UB to supply invalid UTF-8
    pub unsafe fn from_utf8_unchecked(data: MAByteStringBuilder) -> Self {
        Self { inner: data }
    }

    /// fills the string with UTF-8 data, returning an error if it is invalid
    pub fn from_utf8(data: MAByteStringBuilder) -> Result<Self, FromUtf8Error> {
        match str::from_utf8(&data) {
            Ok(..) => Ok( Self { inner: data } ),
            Err(..) => String::from_utf8(data.into_vec()).map(|_| unreachable!()),
        }
    }

    /// fills the string with UTF-8 data, returning an error if it is invalid
    pub fn from_utf8_lossy(data: MAByteStringBuilder) -> Self {
        match str::from_utf8(&data) {
            Ok(..) => Self { inner: data },
            Err(..) => Self::from_string(String::from_utf8_lossy(&data).into_owned()),
        }
    }

    /// convert the MAStringBuilder into a Std::string, this may allocate.
    pub fn into_string(self) -> String {
        unsafe {
            String::from_utf8_unchecked(self.inner.into_vec())
        }
    }

    // convert the MAStringBuilder into a MAByteString
    pub fn into_bytes(self) -> MAByteStringBuilder {
        self.inner
    }

}

impl Deref for MAStringBuilder {
   type Target = str;
   #[inline]
   fn deref(&self) -> &str {
        unsafe {
            str::from_utf8_unchecked(&self.inner)
        }
   }
}

impl DerefMut for MAStringBuilder {
   #[inline]
   fn deref_mut(&mut self) -> &mut str {
        unsafe {
            str::from_utf8_unchecked_mut(&mut self.inner)
        }
   }
}


impl fmt::Display for MAStringBuilder {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl fmt::Debug for MAStringBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(),fmt::Error> {
        fmt::Debug::fmt(self.deref(),f)
    }
}

impl PartialEq for MAStringBuilder {
    fn eq(&self, other : &MAStringBuilder) -> bool {
         return self.deref() == other.deref();
    }
}
impl Eq for MAStringBuilder {}

impl PartialEq<&str> for MAStringBuilder {
    fn eq(&self, other : &&str) -> bool {
         return self.deref() == *other;
    }
}

impl PartialEq<MAStringBuilder> for &str {
    fn eq(&self, other : &MAStringBuilder) -> bool {
         return *self == other.deref();
    }
}


#[test]
fn test_len_transmutation() {
    let v = MAByteString { short: InnerShort { data: [0;SHORTLEN], len: 0x85 } };
    unsafe {
        assert_eq!(v.long.len >> ((size_of::<usize>() - 1) * 8),0x85);
    }
}

#[test]
fn test_reserve_extra_internal() {
    let mut s = MAByteString::from_static(b"test");
    assert_eq!(s.get_mode(),"short");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(10);
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, true);
    assert_eq!(s,b"test");
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s.capacity(),mem::size_of_val(&s)-1);
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(100-s.len());
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"test");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    assert_eq!(s.get_mode(),"cbinline (unique)");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); //should do nothing
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); // no extra space requested, but string must be copied because
                                                  // it's currently in static memory.
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);

    let mut s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"static");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(100-s.len());
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"cbinline (shared)");
    assert_eq!(s2.get_mode(),"cbinline (shared)");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); // no extra space requested, but string must be copied because it's currently
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    //s now has a new buffer
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_eq!(s2.get_mode(),"cbinline (unique)");
    assert!(s2.capacity() >= 100);
    assert!(s2.capacity() <= 150);

    let mut s = MAByteString::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    assert_eq!(s.get_mode(),"unique");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); // should do nothing
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"cbowned (shared)");
    assert_eq!(s2.get_mode(),"cbowned (shared)");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); // no extra space requested, but string must be copied because it's currently
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    //s now has a new buffer
    assert_eq!(s.get_mode(),"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_eq!(s2.get_mode(),"cbowned (unique)");
    assert!(s2.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s2.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    
    let mut s = MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"cbinline (unique)");
    // small reservation, doesn't require reallocation, but does require getting rid
    // of the inline control block
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    unsafe { println!("s.long.len = {}, s.long,cap = {}, s.long.ptr = {:?}",s.long.len,s.long.cap,s.long.ptr); }
    let (ptr, len, isshort) = s.reserve_extra_internal(mem::size_of::<usize>());
    unsafe { println!("s.long.len = {}, s.long,cap = {}, s.long.ptr = {:?}",s.long.len,s.long.cap,s.long.ptr); }
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s.get_mode(),"unique");

    let mut s = MAByteStringBuilder::from_slice(b"test");
    assert_eq!(s.get_mode(),"short");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(10);
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, true);
    assert_eq!(s,b"test");
    assert_eq!(s.get_mode(),"short");
    assert_eq!(s.capacity(),mem::size_of_val(&s)-1);
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(100-s.len());
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"test");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    assert_eq!(s.get_mode(),"unique");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); //should do nothing
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(100-s.len());
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    let s2 = s.clone();
    assert_eq!(s.get_mode(),"unique");
    assert_eq!(s2.get_mode(),"unique");
    
    let mut s = MAByteStringBuilder::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    assert_eq!(s.get_mode(),"unique");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); // should do nothing
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    
    let mut s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_eq!(s.get_mode(),"unique");
    // small reservation, doesn't require reallocation, because of the space reserved for the inline control block
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(b"the quick brown fox jumped over the lazy dog".len()+mem::size_of::<usize>());
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s.get_mode(),"unique");


}