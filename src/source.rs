use std::clone::Clone;
use std::convert::AsRef;
use std::ops::Index;
use std::rc::Rc;
use crate::ubyte;

pub(crate) struct Source<T> {
    inner: Rc<T>,
}

impl<T> Source<T>
where
    T: AsRef<[ubyte]>,
{
    pub(crate) fn new(inner: T) -> Self {
        Self {
            inner: Rc::new(inner),
        }
    }
}

impl<T> Index<usize> for Source<T>
where
    T: AsRef<[ubyte]>,
{
    type Output = ubyte;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> Index<std::ops::Range<usize>> for Source<T>
where
    T: AsRef<[ubyte]>,
{
    type Output = [ubyte];

    fn index(&self, index: std::ops::Range<usize>) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> Index<std::ops::RangeFrom<usize>> for Source<T>
where
    T: AsRef<[ubyte]>,
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

impl<T: AsRef<[ubyte]>> AsRef<[ubyte]> for Source<T> {
    fn as_ref(&self) -> &[ubyte] {
        self.inner.as_ref().as_ref()
    }
}
