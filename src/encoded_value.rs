use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use scroll::ctx;
use scroll::Pread;
use scroll::Uleb128;
use scroll::LE;

use crate::cache::Ref;
use crate::error::Error;
use crate::field::FieldIdItem;
use crate::int;
use crate::jtype::Type;
use crate::jtype::TypeId;
use crate::long;
use crate::method::MethodIdItem;
use crate::method::ProtoIdItem;
use crate::short;
use crate::string::JString;
use crate::string::StringId;
use crate::ubyte;
use crate::uint;
use crate::ulong;
use crate::ushort;
use crate::MethodHandleItem;

#[derive(Debug)]
pub enum EncodedValue {
    Byte(ubyte),
    Short(short),
    Char(ushort),
    Int(int),
    Long(long),
    Type(Type),
    Float(f32),
    Double(f64),
    MethodType(ProtoIdItem),
    MethodHandle(MethodHandleItem),
    String(Ref<JString>),
    Field(FieldIdItem),
    Method(MethodIdItem),
    // TODO: from here..
    Annotation(EncodedAnnotation),
    Array(Vec<EncodedValue>),
    Enum(FieldIdItem),
    Null,
    Boolean(bool),
}

#[derive(FromPrimitive)]
enum ValueType {
    Byte = 0x00,
    Short = 0x02,
    Char = 0x03,
    Int = 0x04,
    Long = 0x06,
    Float = 0x10,
    Double = 0x11,
    MethodType = 0x15,
    MethodHandle = 0x16,
    String = 0x17,
    Type = 0x18,
    Field = 0x19,
    Method = 0x1a,
    Enum = 0x1b,
    Array = 0x1c,
    Annotation = 0x1d,
    Null = 0x1e,
    Boolean = 0x1f,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for EncodedValue
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(
        source: &'a [u8],
        dex: &super::Dex<S>,
    ) -> Result<(Self, Self::Size), Self::Error> {
        let header: ubyte = source.pread(0)?;
        let offset = &mut 0;
        let value_arg = (header >> 5) as usize;
        let value_type = 0b00011111 & header;
        let source = &source[1..1 + value_arg];
        let value = match ValueType::from_u8(value_type).expect("Invalid value type") {
            // TODO: replace debug_assert with errors
            ValueType::Byte => {
                debug_assert_eq!(value_arg, 0);
                EncodedValue::Byte(source.gread(offset)?)
            }
            ValueType::Short => {
                debug_assert!(value_arg < 2);
                EncodedValue::Short(source.gread_with(offset, LE)?)
            }
            ValueType::Char => {
                debug_assert!(value_arg < 2);
                EncodedValue::Char(source.gread_with(offset, LE)?)
            }
            ValueType::Int => {
                debug_assert!(value_arg < 4);
                EncodedValue::Int(source.gread_with(offset, LE)?)
            }
            ValueType::Long => {
                debug_assert!(value_arg < 8);
                EncodedValue::Long(source.gread_with(offset, LE)?)
            }
            ValueType::Float => {
                debug_assert!(value_arg < 4);
                EncodedValue::Float(source.gread_with(offset, LE)?)
            }
            ValueType::Double => {
                debug_assert!(value_arg < 8);
                EncodedValue::Double(source.gread_with(offset, LE)?)
            }
            ValueType::MethodType => {
                debug_assert!(value_arg < 4);
                let proto_id: uint = source.gread_with(offset, LE)?;
                EncodedValue::MethodType(dex.get_proto_item(u64::from(proto_id))?)
            }
            ValueType::MethodHandle => {
                debug_assert!(value_arg < 4);
                let index = source.gread_with::<uint>(offset, LE)?;
                EncodedValue::MethodHandle(dex.get_method_handle_item(index)?)
            }
            ValueType::String => {
                debug_assert!(value_arg < 4);
                let index: uint = source.gread_with(offset, LE)?;
                EncodedValue::String(dex.get_string(index)?)
            }
            ValueType::Type => {
                debug_assert!(value_arg < 4);
                let index: uint = source.gread_with(offset, LE)?;
                EncodedValue::Type(dex.get_type(index)?)
            }
            ValueType::Field => {
                debug_assert!(value_arg < 4);
                let index: uint = source.gread_with(offset, LE)?;
                EncodedValue::Field(dex.get_field_item(ulong::from(index))?)
            }
            ValueType::Method => {
                debug_assert!(value_arg < 4);
                let index = source.gread_with(offset, LE)?;
                EncodedValue::Method(dex.get_method_item(index)?)
            }
            ValueType::Enum => {
                debug_assert!(value_arg < 4);
                let index: uint = source.gread_with(offset, LE)?;
                EncodedValue::Enum(dex.get_field_item(ulong::from(index))?)
            }
            ValueType::Array => {
                debug_assert!(value_arg == 0);
                EncodedValue::Array(source.gread_with::<EncodedArray>(offset, dex)?.into_inner())
            }
            ValueType::Annotation => {
                debug_assert!(value_arg == 0);
                let result = EncodedValue::Annotation(source.gread_with(offset, dex)?);
                result
            }
            ValueType::Null => {
                debug_assert!(value_arg == 0);
                EncodedValue::Null
            }
            ValueType::Boolean => {
                debug_assert!(value_arg < 2);
                EncodedValue::Boolean(value_arg == 1)
            }
        };
        Ok((value, *offset + 1))
    }
}

#[derive(Debug)]
pub struct EncodedArray {
    values: Vec<EncodedValue>,
}

impl EncodedArray {
    fn into_inner(self) -> Vec<EncodedValue> {
        self.values
    }
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for EncodedArray
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let size = Uleb128::read(source, offset)?;
        let values = gread_vec_with!(source, offset, size, ctx);
        Ok((Self { values }, *offset))
    }
}

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
        let elements = gread_vec_with!(source, offset, size, ctx);
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
