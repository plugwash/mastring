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
    }
}

const ITERBLOCKLEN:usize = 8;

//it would be nice to use generics here, but unfortunately it causes
//conflicting implementation errors with the implementations for char
//the type to be implemented must be passed twice, once with any nessacery
//lifetime parameters set to 'a and once without any lifetime parameters.
macro_rules! impl_fromiter_stringlike {
    ($result:ty, $builder:ty,$t:ty,$tplain:ty) => {
        impl<'a> FromIterator<$t> for $result
        {
            fn from_iter<I>(iter: I) -> $result
            where
                I : IntoIterator<Item = $t>
            {
                let mut iter = iter.into_iter();
                const NONE : Option<$tplain> = None;
                let mut block = [NONE;ITERBLOCKLEN];
                let mut result = <$builder>::new();
                let mut i = 0;
                let mut resultlen = 0;
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
                            resultlen += item.as_ref().unwrap().len();
                        }
                        result.reserve(resultlen);
                        for item in block {
                            result += item.as_ref().unwrap();
                        }
                        if end { break }
                        i = 0;
                    }
                }
                result.into()
            }
        }
    }
}

impl_fromiter_bytelike!(MAByteString,MAByteStringBuilder,u8);
impl_fromiter_bytelike!(MAByteString,MAByteStringBuilder,&'a u8);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,&'a [u8],&[u8]);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,Vec<u8>,Vec<u8>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,Box<[u8]>,Box<[u8]>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,Cow<'a,[u8]>,Cow<'_,[u8]>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,CustomCow<'a,MAByteString>,CustomCow<'_,MAByteString>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,CustomCow<'a,MAByteStringBuilder>,CustomCow<'_,MAByteStringBuilder>);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,MAByteString,MAByteString);
impl_fromiter_stringlike!(MAByteString,MAByteStringBuilder,MAByteStringBuilder,MAByteStringBuilder);

impl_fromiter_charlike!(MAString,MAStringBuilder,char);
impl_fromiter_charlike!(MAString,MAStringBuilder,&'a char);
impl_fromiter_stringlike!(MAString,MAStringBuilder,&'a str,&str);
impl_fromiter_stringlike!(MAString,MAStringBuilder,String,String);
impl_fromiter_stringlike!(MAString,MAStringBuilder,Box<str>,Box<str>);
impl_fromiter_stringlike!(MAString,MAStringBuilder,Cow<'a,str>,Cow<'_,str>);
impl_fromiter_stringlike!(MAString,MAStringBuilder,CustomCow<'a,MAString>,CustomCow<'_,MAString>);
impl_fromiter_stringlike!(MAString,MAStringBuilder,CustomCow<'a,MAStringBuilder>,CustomCow<'_,MAStringBuilder>);
impl_fromiter_stringlike!(MAString,MAStringBuilder,MAString,MAString);
impl_fromiter_stringlike!(MAString,MAStringBuilder,MAStringBuilder,MAStringBuilder);

impl_fromiter_bytelike!(MAByteStringBuilder,MAByteStringBuilder,u8);
impl_fromiter_bytelike!(MAByteStringBuilder,MAByteStringBuilder,&'a u8);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,&'a [u8],&[u8]);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,Vec<u8>,Vec<u8>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,Box<[u8]>,Box<[u8]>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,Cow<'a,[u8]>,Cow<'_,[u8]>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,CustomCow<'a,MAByteString>,CustomCow<'_,MAByteString>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,CustomCow<'a,MAByteStringBuilder>,CustomCow<'_,MAByteStringBuilder>);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,MAByteString,MAByteString);
impl_fromiter_stringlike!(MAByteStringBuilder,MAByteStringBuilder,MAByteStringBuilder,MAByteStringBuilder);

impl_fromiter_charlike!(MAStringBuilder,MAStringBuilder,char);
impl_fromiter_charlike!(MAStringBuilder,MAStringBuilder,&'a char);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,&'a str,&str);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,String,String);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,Box<str>,Box<str>);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,Cow<'a,str>,Cow<'_,str>);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,CustomCow<'a,MAString>,CustomCow<'_,MAString>);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,CustomCow<'a,MAStringBuilder>,CustomCow<'_,MAStringBuilder>);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,MAString,MAString);
impl_fromiter_stringlike!(MAStringBuilder,MAStringBuilder,MAStringBuilder,MAStringBuilder);

