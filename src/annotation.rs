use scroll::ctx;
use scroll::Pread;
use scroll::Uleb128;

use crate::encoded_value::EncodedValue;
use crate::error::Error;
use crate::jtype::TypeId;
use crate::string::StringId;

#[derive(Debug)]
pub struct EncodedAnnotation {
    type_idx: TypeId,
    elements: Vec<AnnotationElement>,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for EncodedAnnotation
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let type_idx = Uleb128::read(source, offset)?;
        let type_idx = type_idx as u32;
        let size = Uleb128::read(source, offset)?;
        let elements = try_gread_vec_with!(source, offset, size, ctx);
        Ok((Self { type_idx, elements }, *offset))
    }
}

#[derive(Debug)]
pub struct AnnotationElement {
    name_idx: StringId,
    value: EncodedValue,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for AnnotationElement
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let name_idx = Uleb128::read(source, offset)?;
        let name_idx = name_idx as u32;
        let value = source.gread_with(offset, ctx)?;
        Ok((Self { name_idx, value }, *offset))
    }
}
