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
//! which returns a string representing the current mode.
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

#![no_std]
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
use core::slice;
use core::cmp::max;

extern crate alloc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::str;
use alloc::string::String;
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
    fn from_vec(mut v: Vec<u8>, allowcb: bool) -> Self {
        // it would be nice to use into_raw_parts here, but it's unstable.
        let len = v.len();
        let cap = v.capacity();
        let ptr = v.as_mut_ptr();
        mem::forget(v);
        let mut cbptr : * mut AtomicUsize = ptr::null_mut();
        if allowcb {
            unsafe {
                //check if we have room for a control block.
                //math wont overflow because a vec is limited to isize,
                //which has half the range of usize.
                let end = ptr.add(len);
                let cbstart = len + end.align_offset(align_of::<AtomicUsize>());
                let cbrequired = cbstart + size_of::<AtomicUsize>();
                if cbrequired <= cap {
                    cbptr = ptr.add(cbstart) as * mut AtomicUsize;
                    *cbptr = AtomicUsize::new(3);
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
        //println!("len:{len} veccap:{veccap}");
        let mut v = Vec::with_capacity(veccap);
        v.extend_from_slice(s);
        Self::from_vec(v,allowcb)
    }

    // ensure the pointer is unique
    // if the string is copied, then mincap sets the minimum capacity of the new
    // string, excluding control block space. However if the string is not copied
    // then it's capacity is left unchanged.
    fn make_unique(&mut self, mincap: usize) {
        if self.cap == 0 { // static string, we need to copy
            unsafe {
                *self = InnerLong::from_slice(slice::from_raw_parts(self.ptr,self.len),false,mincap);
            }
        } else {
            let cbptr = self.cbptr.load(Ordering::Relaxed);
            if cbptr.is_null() {
                // we already have unique ownership of the string.
            } else {
                unsafe {
                    let refcount = (*cbptr).load(Ordering::Relaxed) >> 1;
                    if refcount == 1 {
                        // we are the only owner of the String, switch it from shared ownership mode to unique ownership mode.
                        if ((*cbptr).load(Ordering::Relaxed) & 1) == 0 {
                            //free control block pointer
                            let _ = Box::from_raw(cbptr);
                        }
                        self.cbptr.store(ptr::null_mut(),Ordering::Relaxed);
                    } else {
                        // there are other owners, we need to copy
                        *self = InnerLong::from_slice(slice::from_raw_parts(self.ptr,self.len),false,mincap) ;
                    }
                }
            }
        }
    }

    // reallocate the buffer if it's capacity is less than requested.
    // implement an exponential reallocation
    // SAFETY: callers must ensure that the innerlong is in the unique ownership state
    // before calling, use make_unique if needed.
    unsafe fn reserve(&mut self, mut mincap: usize) {
        let cap = self.cap;
        if mincap >= cap { return };
        mincap = max(mincap, cap * 2);
        *self = Self::from_slice(slice::from_raw_parts(self.ptr, self.len), false,  mincap);
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
            MAByteString { long: ManuallyDrop::new(InnerLong::from_vec(v, true)) }
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
                } else if ((*cbptr).load(Ordering::Relaxed) & 1) == 0 {
                    "cbowned"
                } else {
                    "cbinline"
                }
            }
        }

    }

    /// ensure there is capacity for at least mincap bytes
    pub fn reserve(&mut self, mincap: usize) {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                if mincap > SHORTLEN {
                    let mincap = max(mincap,SHORTLEN*2);
                    *self = Self { long: ManuallyDrop::new(InnerLong::from_slice(self,false,mincap)) }
                }
            } else {
                self.long.deref_mut().make_unique(mincap);
                self.long.deref_mut().reserve(mincap);
            }
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
                self.long.deref_mut().make_unique(0);
                // if we get here we are in "unique ownership" mode.
                self.long.ptr
            };
            slice::from_raw_parts_mut(ptr,len)
        }
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
            MAByteStringBuilder { long: ManuallyDrop::new(InnerLong::from_vec(v,false)) }
        }
    }

    pub fn from_mabs(mut s: MAByteString) -> Self {
        unsafe {
            let len = s.long.len;
            if len > isize::max as usize {  //inline string
                MAByteStringBuilder { short: s.short }
            } else {
                s.long.deref_mut().make_unique(0);
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

    /// ensure there is capacity for at least mincap bytes
    pub fn reserve(&mut self, mincap: usize) {
        unsafe {
            let len = self.long.len;
            if len > isize::max as usize {  //inline string
                if mincap > SHORTLEN {
                    let mincap = max(mincap,SHORTLEN*2);
                    *self = Self { long: ManuallyDrop::new(InnerLong::from_slice(self,false,mincap)) }
                }
            } else {
                self.long.deref_mut().reserve(mincap);
            }
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


#[test]
fn test_len_transmutation() {
    let v = MAByteString { short: InnerShort { data: [0;SHORTLEN], len: 0x85 } };
    unsafe {
        assert_eq!(v.long.len >> ((size_of::<usize>() - 1) * 8),0x85);
    }
}

