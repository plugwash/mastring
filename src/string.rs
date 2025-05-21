use alloc::str;
use alloc::string::String;
use alloc::string::FromUtf8Error;
use alloc::fmt;
use alloc::vec::Vec;

use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::Add;
use core::ops::AddAssign;
use core::borrow::Borrow;
use core::hash::Hasher;
use core::hash::Hash;

use crate::MAByteString;
use crate::MAStringBuilder;

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

    pub fn from_builder(b : MAStringBuilder) -> Self {
        MAString { inner: MAByteString::from_builder(b.into_bytes()) }
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

impl Add<&str> for MAString {
    type Output = Self;
    fn add(mut self, rhs: &str) -> Self {
        self += rhs;
        self
    }
}

impl AddAssign<&str> for MAString {
    fn add_assign(&mut self, other: &str) {
        self.inner.add_assign(other.as_bytes());
    }
}

impl Borrow<str> for MAString {
    #[inline]
    fn borrow(&self) -> &str {
        self.deref()
    }
}

impl Hash for MAString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl PartialOrd for MAString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl Ord for MAString {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.deref().cmp(other.deref())
    }
}

