use core::ops::Deref;
use crate::fromiter::Reserve;
use crate::MAString;
use crate::MAStringBuilder;
use crate::MAByteString;
use crate::MAByteStringBuilder;
use crate::CustomCow;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::borrow::Borrow;

const ITERBLOCKLEN:usize = 8;

pub struct JoinableBuf([u8;4]);

/// This trait represents element types that can be joined by the join methods
/// of MAString, MAStringBuilder, MAByteString and MAByteStringBuilder.
/// It is a sealed trait that cannot be used or implemented from outside
/// the mastring crate.
pub trait Joinable<T>
where
    T: ?Sized,
{
    fn join_prepare<'a>(&'a self, buf: &'a mut JoinableBuf) -> &'a T;
}

macro_rules! impl_joiner_simple {
    ($self:ty, $t:ty) => {
         impl Joinable<$t> for $self {
             #[inline]
             fn join_prepare<'a>(&'a self, _buf: &'a mut JoinableBuf) -> &'a $t {
                 self.as_ref()
             }
         }
    }
}

macro_rules! impl_joiner_bytelike {
    ($self:ty, $t:ty) => {
         impl Joinable<$t> for $self {
             #[inline]
             fn join_prepare<'a>(&self, buf: &'a mut JoinableBuf) -> &'a $t {
                 buf.0[0] = *(self.borrow());
                 &buf.0[0..=0]
             }
         }
    }

}

macro_rules! impl_joiner_charlike {
    ($self:ty, $t:ty) => {
         impl Joinable<$t> for $self {
             #[inline]
             fn join_prepare<'a>(&self, buf: &'a mut JoinableBuf) -> &'a $t {
                 self.encode_utf8(&mut buf.0)
             }
         }
    }

}

impl_joiner_bytelike!(u8,[u8]);
impl_joiner_bytelike!(&u8,[u8]);
impl_joiner_simple!(&[u8],[u8]);
impl_joiner_simple!(Vec<u8>,[u8]);
impl_joiner_simple!(Box<[u8]>,[u8]);
impl_joiner_simple!(Cow<'_,[u8]>,[u8]);
impl_joiner_simple!(CustomCow<'_,MAByteString>,[u8]);
impl_joiner_simple!(CustomCow<'_,MAByteStringBuilder>,[u8]);
impl_joiner_simple!(MAByteString,[u8]);
impl_joiner_simple!(MAByteStringBuilder,[u8]);

impl_joiner_charlike!(char,str);
impl_joiner_charlike!(&char,str);
impl_joiner_simple!(&str,str);
impl_joiner_simple!(String,str);
impl_joiner_simple!(Box<str>,str);
impl_joiner_simple!(Cow<'_,str>,str);
impl_joiner_simple!(CustomCow<'_,MAString>,str);
impl_joiner_simple!(CustomCow<'_,MAStringBuilder>,str);
impl_joiner_simple!(MAString,str);
impl_joiner_simple!(MAStringBuilder,str);

pub (super) fn join_internal<B,T,I>(joiner: & <B as Deref>::Target, iter: I) -> B
where
    B: Default + Reserve + Deref + for<'a> core::ops::AddAssign<&'a <B as core::ops::Deref>::Target>,
    I: IntoIterator<Item = T>,
    T: /*Default + */Joinable<<B as Deref>::Target>,
    <B as Deref>::Target: AsRef<[u8]>
{
    let mut iter = iter.into_iter();
    let mut block : [Option<T>;ITERBLOCKLEN] = Default::default();
    let mut i = 0;
    let mut result : B = Default::default();
    let mut resultlen = 0;
    let mut firstc = true;
    let mut firstb = true;

    loop {
        let mut buf = JoinableBuf([0;4]);
        block[i] = iter.next();
        if block[i].is_some() && i < ITERBLOCKLEN-1 {
            i = i + 1;
        } else {
            let (blocklen,end) = if block[i].is_some() {
                (i + 1, false)
            } else {
                (i, true)
            };
            let block = &block[0..blocklen];
            for item in block {
                let item : &T = item.as_ref().unwrap();
                let item : &<B as Deref>::Target = item.join_prepare(&mut buf);
                let item : &[u8] = item.as_ref();
                if firstc {
                    firstc = false;
                } else {
                    resultlen += joiner.as_ref().len();
                }
                resultlen += item.len();
            }
            result.reserve(resultlen);
            for item in block {
                if firstb {
                    firstb = false;
                } else {
                    result += joiner;
                }
                result += item.as_ref().unwrap().join_prepare(&mut buf);
            }
            if end { break }
            i = 0;
        }
    }
    result
}
