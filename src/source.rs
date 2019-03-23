use std::clone::Clone;
use std::convert::AsRef;
use std::ops::Index;
use std::rc::Rc;

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
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner.as_ref().as_ref()[index]
    }
}

impl<T> Index<std::ops::Range<usize>> for Source<T>
where
    T: AsRef<[u8]>,
{
    type Output = [u8];

    fn index(&self, index: std::ops::Range<usize>) -> &Self::Output {
        &self.inner.as_ref().as_ref()[index]
    }
}

impl<T> Index<std::ops::RangeFrom<usize>> for Source<T>
where
    T: AsRef<[u8]>,
{
    type Output = [u8];

    fn index(&self, index: std::ops::RangeFrom<usize>) -> &Self::Output {
        &self.inner.as_ref().as_ref()[index]
    }
}

impl<T> Clone for Source<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: AsRef<[u8]>> AsRef<T> for Source<T> {
    fn as_ref(&self) -> &T {
        self.inner.as_ref()
    }
}
