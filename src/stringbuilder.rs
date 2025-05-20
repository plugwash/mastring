use alloc::str;
use alloc::string::String;
use alloc::string::FromUtf8Error;
use alloc::fmt;
use alloc::vec::Vec;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::Add;
use core::ops::AddAssign;
use crate::MAByteStringBuilder;
use crate::MAString;

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

    // converts a MAString into a MAStringBuilder
    pub fn from_mas(s: MAString) -> Self {
        MAStringBuilder { inner: MAByteStringBuilder::from_mabs(s.into_bytes()) }
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

impl Add<&str> for MAStringBuilder {
    type Output = Self;
    fn add(mut self, rhs: &str) -> Self {
        self += rhs;
        self
    }
}

impl AddAssign<&str> for MAStringBuilder {
    fn add_assign(&mut self, other: &str) {
        self.inner.add_assign(other.as_bytes());
    }
}
