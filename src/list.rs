use scroll::{ctx, Pread};
use std::iter::FromIterator;

use crate::error;
use crate::source::Source;
use scroll::Pread;

#[derive(Debug)]
struct List<T> {
    inner: Vec<T>,
}

struct ListIter<T: AsRef<[u8]>> {
    source: Source<T>,
    offset: usize,
    len: usize
}

impl<T> FromIterator<T> for List<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut result = List::new();
        iter.into_iter().for_each(|s| result.push(s));
        result
    }
}

impl<T> List<T> {
    fn new() -> Self {
        List { inner: Vec::new() }
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.iter()
    }

    fn push(&mut self, val: T) {
        self.inner.push(val);
    }
}

impl<'a, T> ctx::TryFromCtx<'a, super::Endian> for ListIter<T>
where
    T: ctx::TryFromCtx<'a, super::Endian, Size = usize, Error = scroll::Error> + AsRef<[u8]>,
{
    type Error = error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], endian: super::Endian) -> super::Result<(Self, Self::Size)> {
        let mut inner = Vec::with_capacity(context.size);
        let offset = &mut 0;
        for _ in 0..se.size {
            inner.push(
                source
                    .gread_with::<T>(offset, endian)
                    .map_err(error::Error::from)?,
            );
        }
        Ok((List { inner }, *offset))
    }
}
