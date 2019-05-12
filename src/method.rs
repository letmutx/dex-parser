use crate::encoded_item::EncodedItem;
use crate::encoded_item::EncodedItemArray;
use scroll::ctx;
use scroll::Uleb128;

pub struct Method;

pub type MethodId = u64;

pub(crate) struct EncodedMethod {
    pub(crate) method_id: MethodId,
    access_flags: u64,
    code_offset: u64,
}

impl EncodedItem for EncodedMethod {
    fn get_id(&self) -> u64 {
        self.method_id
    }
}

pub(crate) type EncodedMethodArray = EncodedItemArray<EncodedMethod>;

impl<'a> ctx::TryFromCtx<'a, u64> for EncodedMethod {
    type Error = crate::error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], prev_id: u64) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let id = Uleb128::read(source, offset)?;
        let access_flags = Uleb128::read(source, offset)?;
        let code_offset = Uleb128::read(source, offset)?;
        Ok((
            Self {
                method_id: prev_id + id,
                code_offset,
                access_flags,
            },
            *offset,
        ))
    }
}
