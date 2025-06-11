use core::ops::Deref;
use core::fmt;
use core::fmt::Display;

/// CustomCow is similar to alloc::borrow::cow but works the opposite
/// way round, rather than having the "borrowed" type as a type
/// parameter it has the owned type as a type parameter.
pub enum CustomCow<'a, T: Deref> {
    Borrowed(&'a <T as Deref>::Target),
    Owned(T),
}

impl<T: Deref> Deref for CustomCow<'_, T> {
    type Target = <T as Deref>::Target;
    fn deref(&self) -> &<T as Deref>::Target {
        match self {
            Self::Borrowed(b) => { b },
            Self::Owned(o) => { o.deref()},
        }
    }
}

impl<T: Deref> fmt::Display for CustomCow<'_, T>
where
    <T as Deref>::Target: Display
{
    fn fmt(&self,f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.deref().fmt(f)
    }
}

impl<T: Deref> CustomCow<'_, T>
where
    for<'a> &'a <T as Deref>::Target: Into<T>
{
    pub fn into_owned(self) -> T {
        match self {
            Self::Borrowed(b) => { b.into() },
            Self::Owned(o) => { o },
        }
    }
}

impl<T: Deref + Clone> CustomCow<'_, T>
where
    for<'a> &'a <T as Deref>::Target: Into<T>
{
    pub fn to_owned(&self) -> T {
        match self {
            Self::Borrowed(b) => { (*b).into() },
            Self::Owned(o) => { o.clone() },
        }
    }
}

impl<T: Deref + Clone> Clone for CustomCow<'_, T>
{
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(b) => { Self::Borrowed(*b) },
            Self::Owned(o) => { Self::Owned(o.clone()) },
        }
    }
}

