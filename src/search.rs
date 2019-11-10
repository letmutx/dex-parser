use crate::Result;
use scroll::ctx;
use scroll::Pread;
use std::cmp::Ordering;
use std::fmt::Debug;

pub(crate) struct Section<'a> {
    inner: &'a [u8],
}

impl<'a> Section<'a> {
    pub(crate) fn new(inner: &'a [u8]) -> Self {
        Section { inner }
    }

    /// Binary search the contents of this section.
    /// * The items in the section should be of fixed size.
    /// * The items must be sorted in the order that predicate expects.
    pub(crate) fn binary_search<'b, F, T, S, C: Copy>(
        &self,
        element: &'b S,
        ctx: C,
        predicate: F,
    ) -> Result<Option<usize>>
    where
        S: std::fmt::Debug,
        F: Fn(&T, &S) -> Result<Ordering>,
        T: ctx::TryFromCtx<'a, C, Size = usize, Error = scroll::Error> + Debug,
    {
        if self.inner.is_empty() {
            return Ok(None);
        }
        // Figure out the size of one item, all items must be of fixed size
        let mut size = 0;
        let _: T = self.inner.gread_with(&mut size, ctx)?;
        // Number of elements  = Size of buffer / Item size
        let len = self.inner.len() / size;
        debug!(target: "binary-search", "binary-search: item size: {}, buffer length: {}, array length: {}, element: {:?}",
                size, self.inner.len(), len, *element);
        let (mut start, mut end) = (0, len - 1);
        while start < end {
            let mid = start + (end - start) / 2;
            let mid_offset = mid * size;
            let item = self.inner.pread_with(mid_offset, ctx)?;
            let result = predicate(&item, element)?;
            debug!(target: "binary-search", "binary-search: index: {}, item: {:?}, result: {:?}", mid, item, result);
            match result {
                Ordering::Equal => {
                    debug!(target: "binary-search", "binary search: success! index: {}", mid);
                    return Ok(Some(mid));
                }
                Ordering::Less => end = mid - 1,
                Ordering::Greater => start = mid + 1,
            }
        }
        let start_offset = start * size;
        let item = self.inner.pread_with(start_offset, ctx)?;
        Ok(if predicate(&item, element)? == Ordering::Equal {
            debug!(target: "binary-search", "binary search: success! index: {}", start);
            Some(start)
        } else {
            None
        })
    }
}

impl<'a> AsRef<[u8]> for Section<'a> {
    fn as_ref(&self) -> &[u8] {
        self.inner
    }
}
