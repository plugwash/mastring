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
use core::slice;

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

    /// Creates a new MAString with a defined capacity, the resulting
    /// MAString will uniquely own it's buffer.
    pub fn with_capacity(cap: usize) -> Self {
        MAString { inner: MAByteString::with_capacity(cap) }
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
    pub unsafe fn from_utf8_unchecked(data: impl Into<MAByteString>) -> Self {
        let data = data.into();
        Self { inner: data }
    }

    /// fills the string with UTF-8 data, returning an error if it is invalid
    pub fn from_utf8(data: impl Into<MAByteString>) -> Result<Self, FromUtf8Error> {
        let data = data.into();
        match str::from_utf8(&data) {
            Ok(..) => Ok( Self { inner: data } ),
            Err(..) => String::from_utf8(data.into_vec()).map(|_| unreachable!()),
        }
    }

    /// fills the string with UTF-8 data, returning an error if it is invalid
    pub fn from_utf8_lossy(data: impl Into<MAByteString>) -> Self {
        let data = data.into();
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

    // create a MAString from an array of chars, this will allocate if the result
    // will not fit in a short string.
    pub fn from_char_slice(chars: &[char]) -> MAString {
        let mut len = 0;
        for c in chars {
            len += c.len_utf8();
        }
        let mut result = MAByteString::new();
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
        MAString { inner: result }
    }

    // Appends a given slice to the end of this string.
    pub fn push_str(&mut self, string: &str) {
        *self += string;
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

impl fmt::Write for MAString {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        *self += s;
        Ok(())
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

impl From<&str> for MAString {
    #[inline]
    fn from(s : &str) -> Self {
        Self::from_slice(s)
    }
}

impl From<&[char]> for MAString {
    #[inline]
    fn from(s : &[char]) -> Self {
        Self::from_char_slice(s)
    }
}

impl<const N: usize> From<&[char; N]> for MAString {
    #[inline]
    fn from(s : &[char; N]) -> Self {
        Self::from_char_slice(s)
    }
}

impl From<String> for MAString {
    #[inline]
    fn from(s : String) -> Self {
        Self::from_string(s)
    }
}

impl From<MAStringBuilder> for MAString {
    #[inline]
    fn from(s : MAStringBuilder) -> Self {
        Self::from_builder(s)
    }
}

impl From<&String> for MAString {
    #[inline]
    fn from(s : &String) -> Self {
        Self::from_slice(s)
    }
}

impl From<&MAStringBuilder> for MAString {
    #[inline]
    fn from(s : &MAStringBuilder) -> Self {
        Self::from_slice(s)
    }
}

impl From<&MAString> for MAString {
    #[inline]
    fn from(s : &MAString) -> Self {
        s.clone()
    }
}

impl FromIterator<char> for MAString {
    fn from_iter<I>(iter: I) -> MAString
    where
        I : IntoIterator<Item = char>
    {
        let iter = iter.into_iter();

        let mut result = MAStringBuilder::with_capacity(iter.size_hint().0);
        let mut buf = [0u8;4];
        for c in iter {
            result += c.encode_utf8(&mut buf);
        }
        Self::from_builder(result)
    }
}

impl <T> AsRef<T> for MAString 
where
    str: AsRef<T>,
    T: ?Sized,
{
    fn as_ref(&self) -> &T {
        self.deref().as_ref()
    }
}

// helper functions for macro, not intended to be stable API

#[doc(hidden)]
pub const fn chars_utf8len(chars : &[char]) -> usize {
    let mut len = 0;
    let mut i = 0;
    //unfortunately we can't use a for loop in a const fn.
    while i < chars.len() { 
        let c = chars[i];
        len += c.len_utf8();
        i += 1;
    }
    len
}

/// Fills a byte array from a char slice and returns it
/// panics if the byte array is too small.
#[doc(hidden)]
pub const fn chars_to_bytes<const N: usize>(chars : &[char]) -> [u8;N] {
    let mut p = 0;
    let mut result = [0;N];
    let mut i = 0;
    //unfortunately we can't use a for loop in a const fn.
    while i < chars.len() { 
        // we can't use encode_utf8 in a const fn, and to maintain our MSRV
        // so we have to do the encoding manually.
        let c = chars[i] as usize;
        if c < 0x80 {
            result[p] = c as u8;
            p += 1;
        } else if c < 0x800 {
            result[p] = ((c >> 6) + 0b11000000) as u8;
            result[p+1] = ((c & 0b00111111) + 0b10000000) as u8;
            p += 2;
        } else if c < 0x10000 {
            result[p] = ((c >> 12) + 0b11100000) as u8;
            result[p+1] = (((c >> 6) & 0b00111111) + 0b10000000) as u8;
            result[p+2] = ((c & 0b00111111) + 0b10000000) as u8;
            p += 3;
        } else {
            result[p] = ((c >> 18) + 0b11110000) as u8;
            result[p+1] = (((c >> 12) & 0b00111111) + 0b10000000) as u8;
            result[p+2] = (((c >> 6) & 0b00111111) + 0b10000000) as u8;
            result[p+3] = ((c & 0b00111111) + 0b10000000) as u8;
            p += 4;
        }
        i += 1;
    }
    result
}

/// Convenience macro to create a MAString.
///
/// The user may pass byte string literals, array expressions that are
/// compile time constants and have element type char or expressions of type
/// String, MAString or MAByteStringBuilder these will be converted to
/// MAString without the need to allocate.
///
/// The user may also pass expression of types &str, &String
/// and &MAStringBuilder. These will require allocation if the data cannot
/// be stored as a "short string", unfortunately this includes values of type
/// &'static str as there is no way for either a macro or  a generic to
/// distinguish these from other &[u8] values. To efficently create
/// a MAString from a &'static u8 use MAString::from_static instead.
///
/// The user may also pass values of type &MAString, these will require
/// memory allocation if the source MAString is in unique ownership mode
///
/// Passing an array expression that is not a compile time constant will
/// produce errors, to avoid this create a reference to the array.
#[macro_export]
macro_rules! mas {
    ($v:literal) => {
        $crate::MAString::from_static($v)
    };
    ([$($b:expr),+]) => { {
        const chars : &[char] = &[$($b),+];
        const utf8len : usize = $crate::chars_utf8len(chars);
        const bytes : [u8;utf8len] = $crate::chars_to_bytes(chars);
        $crate::MAString::from_static(unsafe { core::str::from_utf8_unchecked(&bytes) })
    } };
    ($v:expr) => {
        $crate::MAString::from($v)
    };
}

crate::customcow::define_customcow_eq!(MAString,str);
