use scroll::{ctx, Pread, Uleb128};

use crate::cache::Ref;
use crate::class::ClassId;
use crate::jtype::Type;
use crate::string::JString;

pub struct Field {
    name: Ref<JString>,
    jtype: Type,
    class: ClassId,
    access_flags: u64,
}

impl Field {
    pub(crate) fn from_dex<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        encoded_field: &EncodedField,
    ) -> super::Result<Self> {
        let field_item = dex.get_field_item(encoded_field.field_id)?;
        Ok(Self {
            name: dex.get_string(field_item.name_idx)?,
            jtype: dex.get_type(field_item.type_idx)?,
            class: field_item.class_idx,
            access_flags: encoded_field.access_flags,
        })
    }
}

#[derive(Pread)]
pub(crate) struct FieldIdItem {
    class_idx: crate::jtype::TypeId,
    type_idx: crate::jtype::TypeId,
    name_idx: crate::string::StringId,
}

impl FieldIdItem {
    pub(crate) fn from_dex<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        offset: u64,
    ) -> super::Result<Self> {
        let source = dex.source.as_ref().as_ref();
        let endian = dex.get_endian();
        Ok(source.pread_with(offset as usize, endian)?)
    }
}

pub type FieldId = u64;

pub(crate) struct EncodedField {
    pub(crate) field_id: FieldId,
    access_flags: u64,
}

impl<'a> ctx::TryFromCtx<'a, u64> for EncodedField {
    type Error = crate::error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], prev_id: u64) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let id = Uleb128::read(source, offset)?;
        let access_flags = Uleb128::read(source, offset)?;
        Ok((
            Self {
                field_id: prev_id + id,
                access_flags,
            },
            *offset,
        ))
    }
}

pub(crate) struct EncodedFieldArray {
    inner: Vec<EncodedField>,
}

impl EncodedFieldArray {
    pub(crate) fn into_iter(self) -> impl Iterator<Item = EncodedField> {
        self.inner.into_iter()
    }
}

pub(crate) struct EncodedFieldArrayCtx<'a, T: AsRef<[u8]>> {
    dex: &'a super::Dex<T>,
    len: usize,
}

impl<'a, T: AsRef<[u8]>> EncodedFieldArrayCtx<'a, T> {
    pub(crate) fn new(dex: &'a super::Dex<T>, len: usize) -> Self {
        EncodedFieldArrayCtx { dex, len }
    }
}

impl<'a, T: AsRef<[u8]>> Copy for EncodedFieldArrayCtx<'a, T> {}

impl<'a, T: AsRef<[u8]>> Clone for EncodedFieldArrayCtx<'a, T> {
    fn clone(&self) -> Self {
        EncodedFieldArrayCtx {
            dex: self.dex,
            len: self.len,
        }
    }
}

impl<'a, T> ctx::TryFromCtx<'a, EncodedFieldArrayCtx<'a, T>> for EncodedFieldArray
where
    T: AsRef<[u8]>,
{
    type Error = crate::error::Error;
    type Size = usize;

    fn try_from_ctx(
        source: &'a [u8],
        ctx: EncodedFieldArrayCtx<'a, T>,
    ) -> super::Result<(Self, Self::Size)> {
        let len = ctx.len;
        let mut prev = 0;
        let offset = &mut 0;
        let mut inner = Vec::with_capacity(len);
        for _ in 0..len {
            let encoded_field: EncodedField = source.gread_with(offset, prev)?;
            prev = encoded_field.field_id;
            inner.push(encoded_field);
        }
        Ok((EncodedFieldArray { inner }, *offset))
    }
}
