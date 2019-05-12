use scroll::ctx;
use scroll::Pread;

pub(crate) trait EncodedItem {
    fn get_id(&self) -> u64;
}

pub(crate) struct EncodedItemArray<T> {
    inner: Vec<T>,
}

impl<T: EncodedItem> EncodedItemArray<T> {
    pub(crate) fn into_iter(self) -> impl Iterator<Item = T> {
        self.inner.into_iter()
    }
}

pub(crate) struct EncodedItemArrayCtx<'a, S: AsRef<[u8]>> {
    dex: &'a super::Dex<S>,
    len: usize,
}

impl<'a, S: AsRef<[u8]>> EncodedItemArrayCtx<'a, S> {
    pub(crate) fn new(dex: &'a super::Dex<S>, len: usize) -> Self {
        Self { dex, len }
    }
}

impl<'a, S: AsRef<[u8]>> Copy for EncodedItemArrayCtx<'a, S> {}

impl<'a, S: AsRef<[u8]>> Clone for EncodedItemArrayCtx<'a, S> {
    fn clone(&self) -> Self {
        Self {
            dex: self.dex,
            len: self.len,
        }
    }
}

impl<'a, S, T: 'a> ctx::TryFromCtx<'a, EncodedItemArrayCtx<'a, S>> for EncodedItemArray<T>
where
    S: AsRef<[u8]>,
    T: EncodedItem + ctx::TryFromCtx<'a, u64, Size = usize, Error = crate::error::Error>,
{
    type Error = crate::error::Error;
    type Size = usize;

    fn try_from_ctx(
        source: &'a [u8],
        ctx: EncodedItemArrayCtx<'a, S>,
    ) -> super::Result<(Self, Self::Size)> {
        let len = ctx.len;
        let mut prev = 0;
        let offset = &mut 0;
        let mut inner = Vec::with_capacity(len);
        for _ in 0..len {
            let encoded_item: T = source.gread_with(offset, prev)?;
            prev = encoded_item.get_id();
            inner.push(encoded_item);
        }
        Ok((EncodedItemArray { inner }, *offset))
    }
}
