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
use core::borrow::Borrow;
use core::hash::Hasher;
use core::hash::Hash;
use core::slice;


#[derive(Clone)]
pub struct MAStringBuilder {
    pub (super) inner: MAByteStringBuilder,
}

impl MAStringBuilder {
    /// Creates a new MAStringBuilder.
    pub const fn new() -> Self {
        MAStringBuilder { inner: MAByteStringBuilder::new() }
    }

    /// Creates a new MAStringBuilder with a defined capacity
    pub fn with_capacity(cap: usize) -> Self {
        MAStringBuilder { inner: MAByteStringBuilder::with_capacity(cap) }
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
    pub unsafe fn from_utf8_unchecked(data: impl Into<MAByteStringBuilder>) -> Self {
        let data = data.into();
        Self { inner: data }
    }

    /// fills the string with UTF-8 data, returning an error if it is invalid
    pub fn from_utf8(data: impl Into<MAByteStringBuilder>) -> Result<Self, FromUtf8Error> {
        let data = data.into();
        match str::from_utf8(&data) {
            Ok(..) => Ok( Self { inner: data } ),
            Err(..) => String::from_utf8(data.into_vec()).map(|_| unreachable!()),
        }
    }

    /// fills the string with UTF-8 data, returning an error if it is invalid
    pub fn from_utf8_lossy(data: impl Into<MAByteStringBuilder>) -> Self {
        let data = data.into();
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


    // create a MAStringBuilder from an array of chars, this will allocate if the result
    // will not fit in a short stringbuilder.
    pub fn from_char_slice(chars: &[char]) -> MAStringBuilder {
        let mut len = 0;
        for c in chars {
            len += c.len_utf8();
        }
        let mut result = MAByteStringBuilder::new();
        unsafe {
            let (mut ptr, _ , short) = result.reserve_extra_internal(len);
            for c in chars {
                let charlen = c.len_utf8();
                c.encode_utf8(slice::from_raw_parts_mut(ptr,charlen));
                ptr = ptr.add(charlen);
            }
            if short {
                result.short_mut().len = (len + 0x80) as u8;
            } else {
                result.long_mut().len = len;
            }
        }
        MAStringBuilder { inner: result }
    }

    // Appends a given slice to the end of this stringbuilder.
    pub fn push_str(&mut self, stringbuilder: &str) {
        *self += stringbuilder;
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

impl fmt::Write for MAStringBuilder {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        *self += s;
        Ok(())
    }
}

impl Borrow<str> for MAStringBuilder {
    #[inline]
    fn borrow(&self) -> &str {
        self.deref()
    }
}

impl Hash for MAStringBuilder {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl PartialOrd for MAStringBuilder {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl Ord for MAStringBuilder {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.deref().cmp(other.deref())
    }
}

impl From<&str> for MAStringBuilder {
    #[inline]
    fn from(s : &str) -> Self {
        Self::from_slice(s)
    }
}

impl From<&[char]> for MAStringBuilder {
    #[inline]
    fn from(s : &[char]) -> Self {
        Self::from_char_slice(s)
    }
}

impl<const N: usize> From<&[char; N]> for MAStringBuilder {
    #[inline]
    fn from(s : &[char; N]) -> Self {
        Self::from_char_slice(s)
    }
}

impl From<String> for MAStringBuilder {
    #[inline]
    fn from(s : String) -> Self {
        Self::from_string(s)
    }
}

impl From<MAString> for MAStringBuilder {
    #[inline]
    fn from(s : MAString) -> Self {
        Self::from_mas(s)
    }
}

impl From<&String> for MAStringBuilder {
    #[inline]
    fn from(s : &String) -> Self {
        Self::from_slice(s)
    }
}

impl From<&MAString> for MAStringBuilder {
    #[inline]
    fn from(s : &MAString) -> Self {
        Self::from_slice(s)
    }
}

impl From<&MAStringBuilder> for MAStringBuilder {
    #[inline]
    fn from(s : &MAStringBuilder) -> Self {
        s.clone()
    }
}

impl <T> AsMut<T> for MAStringBuilder
where
    str: AsMut<T>,
    T: ?Sized,
{
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut().as_mut()
    }
}

impl <T> AsRef<T> for MAStringBuilder
where
    str: AsRef<T>,
    T: ?Sized,
{
    fn as_ref(&self) -> &T {
        self.deref().as_ref()
    }
}

impl Default for MAStringBuilder {
    #[inline]
    fn default() -> MAStringBuilder {
        Self::new()
    }
}

/// Convenience macro to create a MAStringBuilder.
///
/// The user may pass byte stringbuilder literals, array expressions that are
/// compile time constants and have element type char, expressions of type
/// StringBuilder, MAStringBuilder, MAByteString,  &str,  &StringBuilder,
/// &String, &MAStringBuilder.
///
/// Since MAByteStringBuilder does not support shared or static ownership,
/// most uses of this macro will result in memory allocation if the string
/// cannot be represented as a short string. The exception is when
/// converting from a String, a MAStringBuilder or a MAString
/// that is in unique ownership mode. In these cases the existing
/// allocation can be reused.
///
/// Passing an array expression that is not a compile time constant will
/// produce errors, to avoid this create a reference to the array.
#[macro_export]
macro_rules! masb {
    ($v:literal) => {
        $crate::MAStringBuilder::from_slice($v)
    };
    ([$($b:expr),+]) => { {
        const chars : &[char] = &[$($b),+];
        const utf8len : usize = $crate::chars_utf8len(chars);
        const bytes : [u8;utf8len] = $crate::chars_to_bytes(chars);
        $crate::MAStringBuilder::from_slice(unsafe { core::str::from_utf8_unchecked(&bytes) })
    } };
    ($v:expr) => {
        $crate::MAStringBuilder::from($v)
    };
}
