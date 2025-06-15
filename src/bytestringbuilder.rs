use core::sync::atomic::Ordering;
use core::mem::size_of;
use core::mem;
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

extern crate alloc;
use alloc::vec::Vec;
use alloc::str;
use alloc::fmt;
use crate::inner::InnerLong;
use crate::inner::InnerShort;
use crate::inner::InnerNiche;
use crate::MAByteString;
use crate::bytestring::bytes_debug;
use crate::inner::SHORTLEN;

#[cfg(all(miri,test))]
use core::sync::atomic::AtomicPtr;

#[allow(dead_code)]
#[repr(transparent)]
pub struct MAByteStringBuilder {
    inner: InnerNiche,
}
unsafe impl Send for MAByteStringBuilder {}
unsafe impl Sync for MAByteStringBuilder {}


impl MAByteStringBuilder {
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

    /*#[inline]
    pub (super) const fn into_long(self) -> InnerLong {
        unsafe { mem::transmute(self) }
    }*/

    #[inline]
    pub (super) const fn into_short(self) -> InnerShort {
        unsafe { mem::transmute(self) }
    }

    /// Creates a new MAByteStringBuilder.
    /// This will not allocate
    pub const fn new() -> Self {
        Self::from_short( InnerShort { data: [0; SHORTLEN] , len: 0x80 } )
    }

    /// Creates a new MAByteString with a defined capacity
    pub fn with_capacity(cap: usize) -> Self {
        if cap <= SHORTLEN { return Self::new() }
        Self::from_long(InnerLong::from_slice(b"",false,cap))
    }

    /// Creates a MAByteStringBuilder from a slice.
    /// This will allocate if the string cannot be stored as a short string,
    pub fn from_slice(s: &[u8]) -> Self {
        let len = s.len();
        if len <= SHORTLEN {
            let mut data : [u8; SHORTLEN] = [0; SHORTLEN];
            data[0..len].copy_from_slice(&s);
            Self::from_short(InnerShort { data: data, len: len as u8 + 0x80 })
        } else {
            Self::from_long(InnerLong::from_slice(s,false,0))
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
            Self::from_short(InnerShort { data: data, len: len as u8 + 0x80 } )
        } else {
            Self::from_long(InnerLong::from_vec(v,false,0))
        }
    }

    pub fn from_mabs(mut s: MAByteString) -> Self {
        unsafe {
            let len = s.long().len;
            if len > isize::max as usize {  //inline string
                Self::from_short( s.into_short() )
            } else {
                s.long_mut().make_unique(0,false);
                Self::from_long(s.into_long())
            }
        }
    }

    /// Return the current mode of the MAByteStringBuilder (for testing/debugging)
    /// This includes logic to detect states that are valid for MAByteString, but
    /// not for MAByteStringBuilder.
    pub fn get_mode(&self) -> &'static str {
        unsafe {
            let len = self.long().len;
            if len > isize::max as usize {  //inline string
                "short"
            } else if self.long().cap == 0 { // static string
                "static (invalid)"
            } else {
                let cbptr = self.long().cbptr.load(Ordering::Acquire);
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
    pub (super) fn reserve_extra_internal(&mut self, extracap: usize) -> (*mut u8, usize, bool) {
        unsafe {
            let mut len = self.long().len;
            let mincap;
            if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                mincap = len + extracap;
                if mincap > SHORTLEN {
                    let mincap = max(mincap,SHORTLEN*2);
                    *self = Self::from_long(InnerLong::from_slice(slice::from_raw_parts(self.short().data.as_ptr(),len),false,mincap))
                } else {
                    return (self.short_mut().data.as_mut_ptr(),len, true);
                }
            } else {
                mincap = len + extracap;
                self.long_mut().reserve(mincap,false);
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
                    *self = Self::from_long(InnerLong::from_slice(slice::from_raw_parts(self.short().data.as_ptr(),len),false,mincap))
                }
            } else {
                self.long_mut().reserve(mincap,false);
            }
        }
    }

    /// report the "capacity" of the string.
    pub fn capacity(&self) -> usize {
        unsafe {
            let len = self.long().len;
            if len > isize::max as usize {  //inline string
                SHORTLEN
            } else {
                self.long().cap
            }
        }
    }

    // clears the string, retaining it's capacity.
    pub fn clear(&mut self) {
        unsafe {
            let len = self.long().len;
            if len > isize::max as usize {  //inline string
                self.short_mut().len = 0x80;
            } else {
                self.long_mut().len = 0;
            }
        }
    }

    /// convert the MAByteStringBuilder into a Vec, this may allocate.
    pub fn into_vec(self) -> Vec<u8> {
        unsafe {
            let mut len = self.long().len;
            if len > isize::max as usize {  //inline string
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                return slice::from_raw_parts(self.short().data.as_ptr(), len).to_vec();
            }
            let cap = self.long().cap;
            let ptr = self.long().ptr;
            mem::forget(self);
            return Vec::from_raw_parts(ptr,len,cap);
        }
    }

    // Appends a given slice to the end of this bytestringbuilder.
    pub fn push_slice(&mut self, bytestringbuilder: &[u8]) {
        *self += bytestringbuilder;
    }

    // Joins together an iterator of strings, using self as a seperator.
    pub fn join<T,I>(&self, iter : I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: crate::join::Joinable<[u8]>,
    {
        crate::join::join_internal::<MAByteStringBuilder,T,I>(self,iter)
    }
}

impl Drop for MAByteStringBuilder {
    fn drop(&mut self) {
        unsafe {
            let len = self.long().len;
            if len > isize::max as usize { return }; //inline string
            let cap = self.long().cap;
            // we hold the only reference, turn it back into a vec so rust will free it.
            let _ = Vec::from_raw_parts(self.long().ptr, self.long().len, cap);
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

impl DerefMut for MAByteStringBuilder {
   #[inline]
   fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            let mut len = self.long().len;
            let ptr = if len > isize::max as usize {
                len = (len >> ((size_of::<usize>() - 1) * 8)) - 0x80;
                self.short_mut().data.as_mut_ptr()
            } else {
                self.long().ptr
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
                self.short_mut().len = (len + 0x80) as u8;
            } else {
                self.long_mut().len = len;
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

impl Borrow<[u8]> for MAByteStringBuilder {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.deref()
    }
}

impl Hash for MAByteStringBuilder {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl PartialOrd for MAByteStringBuilder {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl Ord for MAByteStringBuilder {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.deref().cmp(other.deref())
    }
}

#[cfg(test)]
macro_rules! assert_mode {
    ($s:expr, $expectedmode:expr) => {
        for _n in 1..=1 { // loop for break, since block labels are not supported in rust 1.63
            let mode = $s.get_mode();
            let expectedmode = $expectedmode;
            #[cfg(miri)]
            if (($s.as_ptr() as usize) & (mem::align_of::<AtomicPtr<usize>>() - 1)) != 0 {
                //miri sometimes gives us unaligned vecs, this can lead to
                //control blocks not fitting inline. This should't break correctness, but
                //it can result in strings being in a different mode from expected.
                if (mode == "unique") && (expectedmode == "cbinline (unique)") { break }
                if (mode == "cbowned (unique)") && (expectedmode == "cbinline (unique)") { break }
                if (mode == "cbowned (shared)") && (expectedmode == "cbinline (shared)") { break }
            }
            assert_eq!(mode,expectedmode);
        }
    }
}

#[test]
fn test_reserve_extra_internal() {
    let mut s = MAByteStringBuilder::from_slice(b"test");
    assert_mode!(s,"short");
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
    assert_mode!(s,"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    assert_mode!(s,"unique");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(0); //should do nothing
    assert_eq!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);

    let mut s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"unique");
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(100-s.len());
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_eq!(s,b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"unique");
    assert!(s.capacity() >= 100);
    assert!(s.capacity() <= 150);
    let s2 = s.clone();
    assert_mode!(s,"unique");
    assert_mode!(s2,"unique");
    
    let mut s = MAByteStringBuilder::from_vec(b"the quick brown fox jumped over the lazy dog".to_vec());
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
    
    let mut s = MAByteStringBuilder::from_slice(b"the quick brown fox jumped over the lazy dog");
    assert_mode!(s,"unique");
    // small reservation, doesn't require reallocation, because of the space reserved for the inline control block
    let oldptr = s.as_ptr();
    let oldlen = s.len();
    let (ptr, len, isshort) = s.reserve_extra_internal(b"the quick brown fox jumped over the lazy dog".len()+mem::size_of::<usize>());
    assert_ne!(ptr as *const u8,oldptr);
    assert_eq!(len,oldlen);
    assert_eq!(isshort, false);
    assert_mode!(s,"unique");


}

impl From<&[u8]> for MAByteStringBuilder {
    #[inline]
    fn from(s : &[u8]) -> Self {
        Self::from_slice(s)
    }
}

impl<const N: usize> From<&[u8;N]> for MAByteStringBuilder {
    #[inline]
    fn from(s : &[u8;N]) -> Self {
        Self::from_slice(s)
    }
}

impl From<Vec<u8>> for MAByteStringBuilder {
    #[inline]
    fn from(s : Vec<u8>) -> Self {
        Self::from_vec(s)
    }
}

impl From<MAByteString> for MAByteStringBuilder {
    #[inline]
    fn from(s : MAByteString) -> Self {
        Self::from_mabs(s)
    }
}

impl From<&Vec<u8>> for MAByteStringBuilder {
    #[inline]
    fn from(s : &Vec<u8>) -> Self {
        Self::from_slice(s)
    }
}

impl From<&MAByteString> for MAByteStringBuilder {
    #[inline]
    fn from(s : &MAByteString) -> Self {
        Self::from_slice(s)
    }
}

impl From<&MAByteStringBuilder> for MAByteStringBuilder {
    #[inline]
    fn from(s : &MAByteStringBuilder) -> Self {
        s.clone()
    }
}

impl <T> AsMut<T> for MAByteStringBuilder
where
    [u8]: AsMut<T>,
    T: ?Sized,
{
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut().as_mut()
    }
}

impl <T> AsRef<T> for MAByteStringBuilder
where
    [u8]: AsRef<T>,
    T: ?Sized,
{
    fn as_ref(&self) -> &T {
        self.deref().as_ref()
    }
}

impl Default for MAByteStringBuilder {
    #[inline]
    fn default() -> MAByteStringBuilder {
        Self::new()
    }
}

/// Convenience macro to create a MAByteStringBuilder.
///
/// The user may pass byte string literals, array expressions that are
/// compile time constants and have element type u8 or expressions of type
/// Vec<u8>, MAByteStringBuilder, MAByteStringBuilder,  &[u8], &[u8;N], 
/// &Vec<u8>, &MABytestring and &MAByteStringBuilder.
///
/// Since MAByteStringBuilder does not support shared or static ownership,
/// most uses of this macro will result in memory allocation if the string
/// cannot be represented as a short string. The exception is when
/// converting from a Vec<u8>, a MAByteStringBuilder or a MAByteString
/// that is in unique ownership mode. In these cases the existing
/// allocation can be reused.
///
/// Passing an array expression that is not a compile time constant will
/// produce errors, to avoid this create a reference to the array.
#[macro_export]
macro_rules! mabsb {
    ($v:literal) => {
        $crate::MAByteStringBuilder::from_slice($v)
    };
    ([$($b:expr),+]) => {
        $crate::MAByteStringBuilder::from_slice({
            const arr: &[u8] = &[$($b),+];
            arr
        })
    };
    ($v:expr) => {
        $crate::MAByteStringBuilder::from($v)
    };
}
