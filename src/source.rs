use std::clone::Clone;
use std::convert::AsRef;
use std::ops::Index;
use std::rc::Rc;

use crate::ubyte;

/// Represents the source `Dex` file. This is a
/// wrapper type that allows for shallow copies
/// of the dex file's source.
pub(crate) struct Source<T> {
    inner: Rc<T>,
}

impl<T> Source<T>
where
    T: AsRef<[u8]>,
{
    pub(crate) fn new(inner: T) -> Self {
        Self {
            inner: Rc::new(inner),
        }
    }
}

impl<T> Index<usize> for Source<T>
where
    T: AsRef<[u8]>,
{
    type Output = ubyte;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> Index<std::ops::Range<usize>> for Source<T>
where
    T: AsRef<[u8]>,
{
    type Output = [ubyte];

    fn index(&self, index: std::ops::Range<usize>) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> Index<std::ops::RangeFrom<usize>> for Source<T>
where
    T: AsRef<[u8]>,
{
    type Output = [ubyte];

    fn index(&self, index: std::ops::RangeFrom<usize>) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> Clone for Source<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: AsRef<[u8]>> AsRef<[u8]> for Source<T> {
    fn as_ref(&self) -> &[ubyte] {
        self.inner.as_ref().as_ref()
    }
}
