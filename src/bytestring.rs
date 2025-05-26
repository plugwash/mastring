use core::sync::atomic::AtomicPtr;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::mem::size_of;
use core::mem;
use core::mem::align_of;
use core::ptr;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::Add;
use core::ops::AddAssign;
use core::slice;
use core::cmp::max;
use core::borrow::Borrow;
use core::hash::Hasher;
use core::hash::Hash;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::str;
use alloc::fmt;
use crate::inner::InnerLong;
use crate::inner::InnerShort;
use crate::inner::InnerNiche;
use crate::MAByteStringBuilder;
use crate::inner::SHORTLEN;


///
#[allow(dead_code)]
#[repr(transparent)]
pub struct MAByteString {
    inner: InnerNiche,
}

unsafe impl Send for MAByteString {}
unsafe impl Sync for MAByteString {}


impl MAByteString {
    #[inline]
    pub (super) const unsafe fn long(&self) -> &InnerLong {
        unsafe { &*(self as *const Self as *const InnerLong) }
    }

    #[inline]
    pub (super) const unsafe fn short(&self) -> &InnerShort {
        unsafe { &*(self as *const Self as *const InnerShort ) }
    }

    #[inline]
    pub (super) unsafe fn long_mut(&mut self) -> &mut InnerLong {
        unsafe { &mut *(self as *mut Self as *mut InnerLong ) }
    }

    #[inline]
    pub (super) unsafe fn short_mut(&mut self) -> &mut InnerShort {
        unsafe { &mut *(self as *mut Self as *mut InnerShort ) }
    }

    #[inline]
    const fn from_long(long : InnerLong) -> Self {
        unsafe { mem::transmute(long) }
    }

    #[inline]
    const fn from_short(short : InnerShort) -> Self {
        unsafe { mem::transmute(short) }
    }

    #[inline]
    pub (super) const fn into_long(self) -> InnerLong {
        unsafe { mem::transmute(self) }
    }

    #[inline]
    pub (super) const fn into_short(self) -> InnerShort {
        unsafe { mem::transmute(self) }
    }

    /// Creates a new MAByteString.
    /// This will not allocate
    pub const fn new() -> Self {
        Self::from_short( InnerShort { data: [0; SHORTLEN] , len: 0x80 } )
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
            Self::from_short( InnerShort { data: data, len: len as u8 + 0x80 } )
        } else {
            Self::from_long( InnerLong::from_slice(s, true,0))
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
            Self::from_short( InnerShort { data: data, len: len as u8 + 0x80 } )
        } else {
            Self::from_long( InnerLong::from_vec(v, true, 0))
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
            Self::from_short( InnerShort { data: data, len: len as u8 + 0x80 } )
        } else {
            Self::from_long( InnerLong { len : len, cap: 0, ptr: s.as_ptr() as *mut u8, cbptr: AtomicPtr::new(ptr::null_mut()) })
        }
    }

    pub fn from_builder(b : MAByteStringBuilder) -> Self {
        unsafe {
            let len = b.long().len;
            if len > isize::max as usize {
                return Self::from_short( b.into_short() );
            }
            let cap = b.long().cap;
            let ptr = b.long().ptr;
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
            Self::from_long(InnerLong { len : len, cap: cap, ptr: ptr, cbptr: AtomicPtr::new(cbptr) })
        }
    }

    /// Return the current mode of the MAByteString (for testing/debugging)
    /// The strings returned from this function are not considred stable, and
    /// changes to them are not considered a semver break.
    pub fn get_mode(&self) -> &'static str {
        unsafe {
            let len = self.long().len;
            if len > isize::max as usize {  //inline string
                "short"
            } else if self.long().cap == 0 { // static string
                "static"
            } else {
                let cbptr = self.long().cbptr.load(Ordering::Acquire);
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
            let mut len = self.long().len;
            let mincap;
            //println!("entering reserve_extra_internal mode={} capacity={}",self.get_mode(),self.capacity());
            if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                mincap = len + extracap;
                if mincap > SHORTLEN {
                    let mincap = max(mincap,SHORTLEN*2);
                    *self = Self::from_long( InnerLong::from_slice(slice::from_raw_parts(self.short().data.as_ptr(),len),true,mincap)) 
                } else {
                    //println!("returning from reserve_extra_internal mode={} capacity={}",self.get_mode(),self.capacity());
                    return (self.short_mut().data.as_mut_ptr(),len, true);
                }
            } else {
                mincap = len + extracap;
                self.long_mut().make_unique(mincap,true);
                //println!("called make_unique mode={} capacity={}",self.get_mode(),self.capacity());
                self.long_mut().reserve(mincap,true);
                //println!("returning from reserve_extra_internal mode={} capacity={}",self.get_mode(),self.capacity());
            }
            // if we reach here, we know it's a "long" String.
            return (self.long().ptr, len, false);
        }
    }

    /// ensure there is capacity for at least mincap bytes
    pub fn reserve(&mut self, mincap: usize) {
        unsafe {
            let mut len = self.long().len;
            if len > isize::max as usize {  //inline string
                if mincap > SHORTLEN {
                    len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                    let mincap = max(mincap,SHORTLEN*2);
                    *self = Self::from_long(InnerLong::from_slice(slice::from_raw_parts(self.short().data.as_ptr(),len),true,mincap))
                }
            } else {
                self.long_mut().make_unique(mincap,true);
                self.long_mut().reserve(mincap,true);
            }
        }
    }

    /// report the "capacity" of the string. Be aware that due to MAByteStrings
    /// copy on write model, this does not gaurantee that future operations
    /// will not allocate. Returns zero for static strings.
    pub fn capacity(&self) -> usize {
        unsafe {
            let len = self.long().len;
            if len > isize::max as usize {  //inline string
                //println!("short string");
                SHORTLEN
            } else {
                //println!("long string, self.long().cap = {}",self.long().cap);
                self.long().usablecap()
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
            let len = self.long().len;
            if len > isize::max as usize {  //inline string
                self.short_mut().len = 0x80;
            } else if self.long().cap == 0 { // static string
                self.short_mut().len = 0x80;
            } else {
                let cbptr = self.long().cbptr.load(Ordering::Relaxed);
                if cbptr.is_null() { // unique ownership mode.
                    self.long_mut().len = 0;
                } else {
                    let refcount = (*cbptr).load(Ordering::Relaxed) >> 1;
                    if refcount == 1 {
                        // we are the only owner of the String
                        self.long_mut().len = 0;
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
            let mut len = self.long().len;
            if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                return slice::from_raw_parts(self.short().data.as_ptr(), len).to_vec();
            }
            self.long_mut().make_unique(0,true);
            let cap = self.long().cap;
            let ptr = self.long().ptr;
            mem::forget(self);
            return Vec::from_raw_parts(ptr,len,cap);
        }
    }
}

impl Drop for MAByteString {
    fn drop(&mut self) {
        unsafe {
            let len = self.long().len;
            if len > isize::max as usize { return }; //inline string
            drop(ptr::read(self.long_mut())); // call drop for the inner type.
        }
    }
}

impl Clone for MAByteString{
    fn clone(&self) -> Self {
        unsafe {
            let len = self.long().len;
            if len > isize::max as usize {  //inline string
                MAByteString::from_short(*self.short())
            } else if self.long().cap == 0 { // static string
                MAByteString::from_long( InnerLong { len : len, cap: 0, ptr: self.long().ptr, cbptr: AtomicPtr::new(ptr::null_mut()) })
            } else {
                let mut cbptr = self.long().cbptr.load(Ordering::Acquire);
                if cbptr.is_null() {
                    let newcbptr = Box::into_raw(Box::new(AtomicUsize::new(2)));
                    if let Err(cxcbptr) = self.long().cbptr.compare_exchange(ptr::null_mut(),newcbptr,Ordering::AcqRel,Ordering::Acquire) {
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
                MAByteString::from_long( InnerLong { len : len, cap: self.long().cap, ptr: self.long().ptr, cbptr: AtomicPtr::new(cbptr) })
            }
        }
    }
}

impl Deref for MAByteString {
   type Target = [u8];
   #[inline]
   fn deref(&self) -> &[u8] {
        unsafe {
            let mut len = self.long().len;
            let ptr = if len > isize::max as usize {
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                self.short().data.as_ptr()
            } else {
                self.long().ptr
            };
            slice::from_raw_parts(ptr,len)
        }
   }
}


impl DerefMut for MAByteString {
   #[inline]
   fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            let mut len = self.long().len;
            let ptr = if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                self.short_mut().data.as_mut_ptr()
            } else { 
                self.long_mut().make_unique(0,true);
                // if we get here we have unique owenership of the data
                // either by being in unique mode, or by being in shared
                // ownership mode but being the only owner.
                self.long().ptr
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
                self.short_mut().len = (len + 0x80) as u8;
            } else {
                self.long_mut().len = len;
            }
        }
    }
}

pub (super) fn bytes_debug(s: &[u8], f: &mut fmt::Formatter<'_>) -> Result<(),fmt::Error> {
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
                    uppernibble += b'0'
                } else {
                    uppernibble -= 10;
                    uppernibble += b'a';
                }
                escaped[2] = uppernibble;
                let mut lowernibble = c & 0xF;
                if lowernibble < 10 {
                    lowernibble += b'0'
                } else {
                    lowernibble -= 10;
                    lowernibble += b'a';
                }
                escaped[3] = lowernibble;
                unsafe {
                    //safety: we know the string is ascii.
                    f.write_str(str::from_utf8_unchecked(&escaped))?;
                }
            }
            groupstart = p + 1;
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

impl Borrow<[u8]> for MAByteString {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.deref()
    }
}

impl Hash for MAByteString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl PartialOrd for MAByteString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl Ord for MAByteString {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.deref().cmp(other.deref())
    }
}

#[cfg(test)]
macro_rules! assert_mode {
    ($s:expr, $expectedmode:expr) => {
        #[allow(unused_labels)] // the label is only used under miri.
        'skipcheck: {
            let mode = $s.get_mode();
            let expectedmode = $expectedmode;
            #[cfg(miri)]
            if (($s.as_ptr() as usize) & (mem::align_of::<AtomicPtr<usize>>() - 1)) != 0 {
                //miri sometimes gives us unaligned vecs, this can lead to
                //control blocks not fitting inline. This should't break correctness, but
                //it can result in strings being in a different mode from expected.
                if (mode == "unique") && (expectedmode == "cbinline (unique)") { break 'skipcheck }
                if (mode == "cbowned (unique)") && (expectedmode == "cbinline (unique)") { break 'skipcheck }
                if (mode == "cbowned (shared)") && (expectedmode == "cbinline (shared)") { break 'skipcheck }
            }
            assert_eq!(mode,expectedmode);
        }
    }
}

#[test]
fn test_len_transmutation() {
    let v = MAByteString::from_short(InnerShort { data: [0;SHORTLEN], len: 0x85 } );
    unsafe {
        assert_eq!(v.long().len >> ((size_of::<usize>() - 1) * 8),0x85);
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
    assert_mode!(s,"short");
    assert_eq!(s.capacity(),mem::size_of_val(&s)-1);
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(100-s.len());
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"test");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    assert_mode!(s,"cbinline (unique)");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); //should do nothing
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); // no extra space requested, but string must be copied because
                                                  // it's currently in static memory.
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);

    let mut s = MAByteString::from_static(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"static");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(100-s.len());
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    let s2 = s.clone();
    assert_mode!(s,"cbinline (shared)");
    assert_mode!(s2,"cbinline (shared)");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); // no extra space requested, but string must be copied because it's currently
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    //s now has a new buffer
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_mode!(s2,"cbinline (unique)");
    assert!(s2.capacity() >= 100);
    assert!(s2.capacity() <= 150);

    let mut s = MAByteString::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
    assert_mode!(s,"unique");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); // should do nothing
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"unique");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    let s2 = s.clone();
    assert_mode!(s,"cbowned (shared)");
    assert_mode!(s2,"cbowned (shared)");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); // no extra space requested, but string must be copied because it's currently
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    //s now has a new buffer
    assert_mode!(s,"cbinline (unique)");
    assert!(s.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    //s2 now owns the buffer fomerly owned by s
    assert_mode!(s2,"cbowned (unique)");
    assert!(s2.capacity() >= "the quick brown fox jumped over the lazy dog".len());
    assert!(s2.capacity() <= "the quick brown fox jumped over the lazy dog".len() + 50);
    
    let mut s = MAByteString::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"cbinline (unique)");
    // small reservation, doesn't require reallocation, but does require getting rid
    // of the inline control block
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    //unsafe { println!("s.long.len = {}, s.long,cap = {}, s.long.ptr = {:?}",s.long.len,s.long.cap,s.long.ptr); }
    let (ptr, len, isshort) = s.reserve_extra_internal(mem::size_of::<usize>());
    //unsafe { println!("s.long.len = {}, s.long,cap = {}, s.long.ptr = {:?}",s.long.len,s.long.cap,s.long.ptr); }
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_mode!(s,"unique");

    
}
