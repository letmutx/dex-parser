use scroll::{ctx, Pread, Uleb128};

use crate::cache::Ref;
use crate::class::ClassId;
use crate::encoded_item::EncodedItem;
use crate::encoded_item::EncodedItemArray;
use crate::error::Error;
use crate::jtype::Type;
use crate::string::JString;
use crate::string::StringId;
use crate::uint;
use crate::ulong;
use crate::ushort;
use crate::FieldAccessFlags;
use getset::{CopyGetters, Getters};

/// Represents the field of a class
#[derive(Debug, Getters, CopyGetters)]
pub struct Field {
    /// Name of the field.
    #[get = "pub"]
    name: Ref<JString>,
    /// Type of the field.
    #[get = "pub"]
    jtype: Type,
    /// Class which this field belongs to.
    #[get_copy = "pub"]
    class: ClassId,
    /// Access flags for the field.
    #[get_copy = "pub"]
    access_flags: FieldAccessFlags,
}

impl Field {
    pub(crate) fn try_from_dex<S: AsRef<[u8]>>(
        dex: &super::Dex<S>,
        encoded_field: &EncodedField,
    ) -> super::Result<Self> {
        debug!(target: "field", "encoded field: {:?}", encoded_field);
        let field_item = dex.get_field_item(encoded_field.field_id)?;
        debug!(target: "field", "field id item: {:?}", field_item);
        Ok(Self {
            name: dex.get_string(field_item.name_idx)?,
            jtype: dex.get_type(uint::from(field_item.type_idx))?,
            class: uint::from(field_item.class_idx),
            access_flags: FieldAccessFlags::from_bits(encoded_field.access_flags).ok_or_else(
                || {
                    Error::InvalidId(format!(
                        "Invalid access flags when loading field {}",
                        field_item.name_idx
                    ))
                },
            )?,
        })
    }
}

pub(crate) type EncodedFieldArray = EncodedItemArray<EncodedField>;

/// https://source.android.com/devices/tech/dalvik/dex-format#field-id-item
#[derive(Pread, Debug)]
pub struct FieldIdItem {
    class_idx: ushort,
    type_idx: ushort,
    name_idx: StringId,
}

impl FieldIdItem {
    pub(crate) fn try_from_dex<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        offset: ulong,
    ) -> super::Result<Self> {
        let source = &dex.source;
        Ok(source.pread_with(offset as usize, dex.get_endian())?)
    }
}

pub type FieldId = ulong;

/// https://source.android.com/devices/tech/dalvik/dex-format#encoded-field-format
#[derive(Debug)]
pub(crate) struct EncodedField {
    pub(crate) field_id: FieldId,
    access_flags: ulong,
}

impl EncodedItem for EncodedField {
    fn id(&self) -> ulong {
        self.field_id
    }
}

impl<'a> ctx::TryFromCtx<'a, ulong> for EncodedField {
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], prev_id: ulong) -> super::Result<(Self, Self::Size)> {
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
