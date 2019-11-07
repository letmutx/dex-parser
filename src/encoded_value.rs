use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use scroll::ctx;
use scroll::Pread;
use scroll::Uleb128;
use scroll::LE;

use crate::annotation::EncodedAnnotation;
use crate::cache::Ref;
use crate::error::Error;
use crate::field::FieldIdItem;
use crate::int;
use crate::jtype::Type;
use crate::long;
use crate::method::MethodHandleItem;
use crate::method::MethodIdItem;
use crate::method::ProtoIdItem;
use crate::short;
use crate::string::JString;
use crate::ubyte;
use crate::uint;
use crate::ulong;
use crate::ushort;
use crate::Result;

/// Used to represent values of fields, annotations etc.
/// https://source.android.com/devices/tech/dalvik/dex-format#encoding
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
    Annotation(EncodedAnnotation),
    Array(Vec<EncodedValue>),
    Enum(FieldIdItem),
    Null,
    Boolean(bool),
}

/// https://source.android.com/devices/tech/dalvik/dex-format#value-formats
#[derive(FromPrimitive, Debug)]
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

macro_rules! try_zero_extended_gread {
    ($source:expr,$offset:expr,$value_arg:expr,$size:expr) => {{
        let mut bytes = [0x0; $size];
        for (i, value) in $source[1..=$value_arg].iter().enumerate() {
            bytes[i] = *value;
        }
        let value = bytes.pread_with(0, LE)?;
        *$offset += 1 + $value_arg;
        value
    }};
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for EncodedValue
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], dex: &super::Dex<S>) -> Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let header: ubyte = source.gread(offset)?;
        let value_arg = (header >> 5) as usize;
        let value_type = 0b0001_1111 & header;
        let value_type = ValueType::from_u8(value_type)
            .ok_or_else(|| Error::InvalidId(format!("Invalid value type {}", value_type)))?;
        debug!(target: "encoded-value", "encoded value type: {:?}", value_type);
        let value = match value_type {
            ValueType::Byte => {
                debug_assert_eq!(value_arg, 0);
                EncodedValue::Byte(try_zero_extended_gread!(source, offset, value_arg, 1))
            }
            ValueType::Short => {
                debug_assert!(value_arg < 2);
                EncodedValue::Short(try_zero_extended_gread!(source, offset, value_arg, 2))
            }
            ValueType::Char => {
                debug_assert!(value_arg < 2);
                EncodedValue::Char(try_zero_extended_gread!(source, offset, value_arg, 2))
            }
            ValueType::Int => {
                debug_assert!(value_arg < 4);
                EncodedValue::Int(try_zero_extended_gread!(source, offset, value_arg, 4))
            }
            ValueType::Long => {
                debug_assert!(value_arg < 8);
                EncodedValue::Long(try_zero_extended_gread!(source, offset, value_arg, 8))
            }
            ValueType::Float => {
                debug_assert!(value_arg < 4);
                EncodedValue::Float(try_zero_extended_gread!(source, offset, value_arg, 4))
            }
            ValueType::Double => {
                debug_assert!(value_arg < 8);
                EncodedValue::Double(try_zero_extended_gread!(source, offset, value_arg, 8))
            }
            ValueType::MethodType => {
                debug_assert!(value_arg < 4);
                let proto_id: uint = try_zero_extended_gread!(source, offset, value_arg, 4);
                EncodedValue::MethodType(dex.get_proto_item(u64::from(proto_id))?)
            }
            ValueType::MethodHandle => {
                debug_assert!(value_arg < 4);
                let index: uint = try_zero_extended_gread!(source, offset, value_arg, 4);
                EncodedValue::MethodHandle(dex.get_method_handle_item(index)?)
            }
            ValueType::String => {
                debug_assert!(value_arg < 4);
                let index: uint = try_zero_extended_gread!(source, offset, value_arg, 4);
                EncodedValue::String(dex.get_string(index)?)
            }
            ValueType::Type => {
                debug_assert!(value_arg < 4);
                let index: uint = try_zero_extended_gread!(source, offset, value_arg, 4);
                EncodedValue::Type(dex.get_type(index)?)
            }
            ValueType::Field => {
                debug_assert!(value_arg < 4);
                let index: uint = try_zero_extended_gread!(source, offset, value_arg, 4);
                EncodedValue::Field(dex.get_field_item(ulong::from(index))?)
            }
            ValueType::Method => {
                debug_assert!(value_arg < 4);
                let index: uint = try_zero_extended_gread!(source, offset, value_arg, 4);
                EncodedValue::Method(dex.get_method_item(ulong::from(index))?)
            }
            ValueType::Enum => {
                debug_assert!(value_arg < 4);
                let index: uint = try_zero_extended_gread!(source, offset, value_arg, 4);
                EncodedValue::Enum(dex.get_field_item(ulong::from(index))?)
            }
            ValueType::Array => {
                debug_assert!(value_arg == 0);
                let encoded_array: EncodedArray = source.gread_with(offset, dex)?;
                EncodedValue::Array(encoded_array.into_inner())
            }
            ValueType::Annotation => {
                debug_assert!(value_arg == 0);
                EncodedValue::Annotation(source.gread_with(offset, dex)?)
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
        Ok((value, *offset))
    }
}

/// Array of `EncodedValue`s
#[derive(Debug)]
pub struct EncodedArray {
    values: Vec<EncodedValue>,
}

impl EncodedArray {
    pub(crate) fn into_inner(self) -> Vec<EncodedValue> {
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
        // TODO: find out why try_gread_vec_with! doesn't work here: fails in scroll
        debug!(target: "encoded-array", "encoded array size: {}", size);
        let mut values = Vec::with_capacity(size as usize);
        for _ in 0..size {
            values.push(source.gread_with(offset, ctx)?);
        }
        Ok((Self { values }, *offset))
    }
}
