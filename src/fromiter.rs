use crate::MAString;
use crate::MAStringBuilder;
use crate::CustomCow;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use crate::MAByteString;
use crate::MAByteStringBuilder;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::ops::Deref;

macro_rules! impl_fromiter_bytelike {
    ($result:ty, $builder:ty, $t:ty) => {
        impl<'a> FromIterator<$t> for $result {
            fn from_iter<I>(iter: I) -> $result
            where
                I : IntoIterator<Item = $t>
            {
                let iter = iter.into_iter();

                let mut result = MAByteStringBuilder::with_capacity(iter.size_hint().0);
                for c in iter {
                   result += &[*c.borrow()];
                }
                result.into()
            }
        }

        impl<'a> Extend<$t> for $result 
        {
            fn extend<I>(&mut self, iter: I)
                where I: IntoIterator<Item = $t>
            {
                let iter = iter.into_iter();

                self.reserve_extra_internal(iter.size_hint().0);
                for c in iter {
                    *self += &[*c.borrow()];
                }
            }
        }

    }
}

macro_rules! impl_fromiter_charlike {
    ($result:ty, $builder:ty, $t:ty) => {
        impl<'a> FromIterator<$t> for $result {
            fn from_iter<I>(iter: I) -> $result
            where
                I : IntoIterator<Item = $t>
            {
                let iter = iter.into_iter();

                let mut result = <$builder>::with_capacity(iter.size_hint().0);
                let mut buf = [0u8;4];
                for c in iter {
                    result += c.encode_utf8(&mut buf);
                }
                result.into()
            }
        }
        impl<'a> Extend<$t> for $result 
        {
            fn extend<I>(&mut self, iter: I)
                where I: IntoIterator<Item = $t>
            {
                let iter = iter.into_iter();

                self.inner.reserve_extra_internal(iter.size_hint().0);
                let mut buf = [0u8;4];
                for c in iter {
                    *self += c.encode_utf8(&mut buf);
                }
            }
        }

    }
}

const ITERBLOCKLEN:usize = 8;

trait Reserve {
    fn reserve(&mut self, cap: usize);
}

macro_rules! impl_reserve {
    ($t:ty) => {
        impl Reserve for $t {
            #[inline]
            fn reserve(&mut self, cap: usize) {
                self.reserve(cap);
            }
        }
    }
}

impl_reserve!(MAByteString);
impl_reserve!(MAString);
impl_reserve!(MAByteStringBuilder);
impl_reserve!(MAStringBuilder);

#[inline]
fn from_iter_stringlike_core<B,T,I>(result : &mut B,iter: I)
where
    B: Default + Reserve + Deref + for<'a> core::ops::AddAssign<&'a <B as core::ops::Deref>::Target>,
    I: IntoIterator<Item = T>,
    T: Default + AsRef<<B as Deref>::Target>,
    <B as Deref>::Target: AsRef<[u8]>
{
    let mut iter = iter.into_iter();
    let mut block : [Option<T>;ITERBLOCKLEN] = Default::default();
    let mut i = 0;
    let mut resultlen = result.as_ref().len();
    loop {
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
                let item : &<B as Deref>::Target = item.as_ref();
                let item : &[u8] = item.as_ref();
                resultlen += item.len();
            }
            result.reserve(resultlen);
            for item in block {
                *result += item.as_ref().unwrap().as_ref();
            }
            if end { break }
            i = 0;
        }
    }
}

//it would be nice to use generics here, but unfortunately it causes
//conflicting implementation errors with the implementations for char
//the type to be implemented must be passed twice, once with any nessacery
//lifetime parameters set to 'a and once without any lifetime parameters.
macro_rules! impl_fromiter_stringlike {
    ($result:ty, $builder:ty,$t:ty) => {
        impl<'a> FromIterator<$t> for $result
        {
            fn from_iter<I>(iter: I) -> $result
            where
                I : IntoIterator<Item = $t>
            {
                let mut result = <$builder>::new();
                from_iter_stringlike_core::<$builder,$t,I>(&mut result,iter);
                result.into()
            }
        }

        impl<'a> Extend<$t> for $result 
        {
            fn extend<I>(&mut self, iter: I)
                where I: IntoIterator<Item = $t>
            {
                from_iter_stringlike_core::<$result,$t,I>(self,iter)
            }
        }
    }
}

impl_fromiter_bytelike!(MAByteString,MAByteStringBuilder,u8);
impl_fromiter_bytelike!(MAByteString,MAByteStringBuilder,&'a u8);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,&'a [u8]);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,Vec<u8>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,Box<[u8]>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,Cow<'a,[u8]>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,CustomCow<'a,MAByteString>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,CustomCow<'a,MAByteStringBuilder>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,MAByteString);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,MAByteStringBuilder);

impl_fromiter_charlike!(MAString,MAStringBuilder,char);
impl_fromiter_charlike!(MAString,MAStringBuilder,&'a char);
impl_fromiter_stringlike!(MAString,MAStringBuilder,&'a str);
impl_fromiter_stringlike!(MAString,MAStringBuilder,String);
impl_fromiter_stringlike!(MAString,MAStringBuilder,Box<str>);
impl_fromiter_stringlike!(MAString,MAStringBuilder,Cow<'a,str>);
impl_fromiter_stringlike!(MAString,MAStringBuilder,CustomCow<'a,MAString>);
impl_fromiter_stringlike!(MAString,MAStringBuilder,CustomCow<'a,MAStringBuilder>);
impl_fromiter_stringlike!(MAString,MAStringBuilder,MAString);
impl_fromiter_stringlike!(MAString,MAStringBuilder,MAStringBuilder);

impl_fromiter_bytelike!(MAByteStringBuilder,MAByteStringBuilder,u8);
impl_fromiter_bytelike!(MAByteStringBuilder,MAByteStringBuilder,&'a u8);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,&'a [u8]);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,Vec<u8>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,Box<[u8]>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,Cow<'a,[u8]>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,CustomCow<'a,MAByteString>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,CustomCow<'a,MAByteStringBuilder>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,MAByteString);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,MAByteStringBuilder);

impl_fromiter_charlike!(MAStringBuilder,MAStringBuilder,char);
impl_fromiter_charlike!(MAStringBuilder,MAStringBuilder,&'a char);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,&'a str);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,String);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,Box<str>);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,Cow<'a,str>);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,CustomCow<'a,MAString>);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,CustomCow<'a,MAStringBuilder>);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,MAString);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,MAStringBuilder);

