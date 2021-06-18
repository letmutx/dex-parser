//! Dex `Field` and supporting structures
use scroll::{ctx, Pread, Uleb128};

use crate::{
    annotation::AnnotationSetItem,
    class::ClassId,
    encoded_item::{EncodedItem, EncodedItemArray},
    encoded_value::EncodedValue,
    error::Error,
    jtype::{Type, TypeId},
    string::{DexString, StringId},
    ulong, ushort, utils,
};
use getset::{CopyGetters, Getters};

bitflags! {
    /// Access flags of a Field.
    pub struct AccessFlags: ulong {
        const PUBLIC = 0x1;
        const PRIVATE = 0x2;
        const PROTECTED = 0x4;
        const STATIC = 0x8;
        const FINAL = 0x10;
        const VOLATILE = 0x40;
        const TRANSIENT = 0x80;
        const SYNTHETIC = 0x1000;
        const ENUM = 0x4000;
    }
}

/// Represents the field of a class
#[derive(Debug, Getters, CopyGetters)]
pub struct Field {
    /// Name of the field.
    #[get = "pub"]
    name: DexString,
    /// Type of the field.
    #[get = "pub"]
    jtype: Type,
    /// Class which this field belongs to.
    #[get_copy = "pub"]
    class: ClassId,
    /// Access flags for the field.
    #[get_copy = "pub"]
    access_flags: AccessFlags,
    /// Initial value of the field. Always `None` for non-static fields.
    /// If the value is `None`, it is not guaranteed that initial_value is `null`
    /// at runtime. The field might be initialized in `<clinit>` method.
    initial_value: Option<EncodedValue>,
    /// Annotations of the field.
    #[get = "pub"]
    annotations: AnnotationSetItem,
    /// `FieldId` of the field.
    #[get = "pub"]
    id: FieldId,
}

impl Field {
    /// Initial value of the field. Always `None` for non-static fields.
    /// If the value is `None`, it is not guaranteed that initial_value is `null`
    /// at runtime. The field might be initialized in `<clinit>` method.
    pub fn initial_value(&self) -> Option<&EncodedValue> {
        self.initial_value.as_ref()
    }

    gen_is_flag_set!(is_public, PUBLIC);
    gen_is_flag_set!(is_private, PRIVATE);
    gen_is_flag_set!(is_protected, PROTECTED);
    gen_is_flag_set!(is_static, STATIC);
    gen_is_flag_set!(is_final, FINAL);
    gen_is_flag_set!(is_volatile, VOLATILE);
    gen_is_flag_set!(is_transient, TRANSIENT);
    gen_is_flag_set!(is_synthetic, SYNTHETIC);
    gen_is_flag_set!(is_enum, ENUM);

    /// Returns the value of `dalvik.annotation.Signature`.
    pub fn signature(&self) -> super::Result<Option<String>> {
        utils::get_signature(self.annotations())
    }

    pub(crate) fn try_from_dex<S: AsRef<[u8]>>(
        dex: &super::Dex<S>,
        encoded_field: &EncodedField,
        initial_value: Option<EncodedValue>,
        annotations: AnnotationSetItem,
    ) -> super::Result<Self> {
        debug!(target: "field", "encoded field: {:?}", encoded_field);
        let field_item = dex.get_field_item(encoded_field.field_id)?;
        debug!(target: "field", "field id item: {:?}", field_item);
        Ok(Self {
            name: dex.get_string(field_item.name_idx)?,
            jtype: dex.get_type(TypeId::from(field_item.type_idx))?,
            class: ClassId::from(field_item.class_idx),
            access_flags: AccessFlags::from_bits(encoded_field.access_flags).ok_or_else(|| {
                Error::InvalidId(format!(
                    "Invalid access flags when loading field {}",
                    field_item.name_idx
                ))
            })?,
            initial_value,
            annotations,
            id: encoded_field.field_id,
        })
    }
}

/// List of `EncodedField`s
pub type EncodedFieldArray = EncodedItemArray<EncodedField>;

#[derive(Pread)]
struct FieldIdData {
    class_idx: ushort,
    type_idx: ushort,
    name_idx: StringId,
}

/// Defines a `Field`
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#field-id-item)
#[derive(Debug, Getters, PartialEq)]
#[get = "pub"]
pub struct FieldIdItem {
    /// Index into `TypeId`s list which contains the defining class's `Type`.
    class_idx: ushort,
    /// Index into `TypeId`s list which contains the `Type` of the field.
    type_idx: ushort,
    /// Index into `StringId`s list which contains the name of the field.
    name_idx: StringId,
    /// `FieldId` of this field.
    id: FieldId,
}

impl FieldIdItem {
    pub(crate) fn try_from_dex<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        offset: ulong,
        field_id: FieldId,
    ) -> super::Result<Self> {
        let source = &dex.source;
        let field: FieldIdData = source.pread_with(offset as usize, dex.get_endian())?;
        Ok(FieldIdItem {
            class_idx: field.class_idx,
            type_idx: field.type_idx,
            name_idx: field.name_idx,
            id: field_id,
        })
    }
}

/// Index into the `FieldId`s list.
pub type FieldId = ulong;

/// Contains a `FieldId` along with its access flags.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#encoded-field-format)
#[derive(Debug, CopyGetters)]
#[get_copy = "pub"]
pub struct EncodedField {
    /// Index into the `FieldId`s list for the identity of this field represented as
    /// a difference from the index of previous element in the list.
    pub(crate) field_id: FieldId,
    /// Access flags for the field.
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
