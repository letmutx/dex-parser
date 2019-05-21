use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use scroll::ctx;
use scroll::Pread;
use scroll::LE;

use crate::cache::Ref;
use crate::error::Error;
use crate::int;
use crate::jtype::Type;
use crate::long;
use crate::short;
use crate::string::JString;
use crate::ubyte;
use crate::uint;
use crate::ushort;
use crate::method::ProtoIdItem;
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
    Field(FieldProto),
    Method(MethodProto),
    // TODO: from here..
    Annotation(EncodedAnnotation),
    Array(Vec<EncodedValue>),
    Enum(FieldProto),
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
        let mut size_read = 1;
        let value_arg = (header >> 5) as usize;
        let value_type = 0b00011111 & header;
        let value = match ValueType::from_u8(value_type) {
            ValueType::Byte => {
                debug_assert_eq!(value_arg, 0);
                size_read += value_arg + 1;
                EncodedValue::Byte(source.pread(1)?)
            }
            ValueType::Short => {
                debug_assert!(value_arg < 2);
                size_read += value_arg + 1;
                let mut bytes: [ubyte; 2] = [0x00, 0x00];
                for (i, value) in source[1..1 + value_arg].iter().enumerate() {
                    bytes[i] = *value;
                }
                EncodedValue::Short(bytes.pread_with(0, LE)?)
            }
            ValueType::Char => {
                debug_assert!(value_arg < 2);
                size_read += value_arg + 1;
                EncodedValue::Char(source[1..1+value_arg].pread_with(0, LE)?)
            }
            ValueType::Int => {
                debug_assert!(value_arg < 4);
                size_read += value_arg + 1;
                EncodedValue::Int(source[1..1+value_arg].pread_with(0, LE)?)
            }
            ValueType::Long => {
                debug_assert!(value_arg < 8);
                size_read += value_arg + 1;
                EncodedValue::Long(source[1..1+value_arg].pread_with(0, LE)?)
            }
            ValueType::Float => {
                debug_assert!(value_arg < 4);
                size_read += value_arg + 1;
                EncodedValue::Float(source[1..1 + value_arg].pread_with(0, LE)?)
            }
            ValueType::Double => {
                debug_assert!(value_arg < 8);
                size_read += value_arg + 1;
                EncodedValue::Double(source[1..1 + value_arg].pread_with(0, LE)?)
            }
            ValueType::MethodType => {
                debug_assert!(value_arg < 4);
                size_read += value_arg + 1;
                let proto_id: uint = source[1..1 + value_arg].pread_with(0, LE)?;
                EncodedValue::MethodType(dex.get_proto_item(u64::from(proto_id))?)
            }
            ValueType::MethodHandle => {
                debug_assert!(value_arg < 4);
                size_read += value_arg + 1;
                let index = source[1..1 + value_arg].pread_with::<uint>(0, LE)?;
                EncodedValue::MethodHandle(dex.get_method_handle_item(index)?)
            },
            ValueType::String => {
                debug_assert!(value_arg < 4);
                size_read += value_arg + 1;
                let index: uint = source[1..1 + value_arg].pread_with(0, LE)?;
                EncodedValue::String(dex.get_string(index)?)
            }
            ValueType::Type => {
                debug_assert!(value_arg < 4);
                size_read += value_arg + 1;
                let index: uint = source[1..1 + value_arg].pread_with(0, LE)?;
                EncodedValue::Type(dex.get_type(index)?)
            }
            ValueType::Field => {
                debug_assert!(value_arg < 4);
                size_read += value_arg + 1;
                let index: uint = source[1..1 + value_arg].pread_with(0, LE)?;
                EncodedValue::Field(dex.get_field_item(ulong::from(index))?)
            }
            ValueType::Method => {
                debug_assert!(value_arg < 4);
                size_read += value_arg + 1;
                let index = source[1..1 + value_arg].pread_with(0, LE)?;
                EncodedValue::Method(dex.get_method_item(index)?)
            }
            ValueType::Enum => {
                debug_assert!(value_arg < 4);
                size_read += value_arg + 1;
                let index: uint = source[1..1 + value_arg].pread_with(0, LE)?;
                EncodedValue::Enum(dex.get_field_item(ulong::from(index))?)
            }
            ValueType::Array => {
                debug_assert!(value_arg == 0);
                let offset = &mut 1;
                size_read += *offset;
                EncodedValue::Array(
                    source.gread_with::<EncodedArray>(offset, dex)?.into_inner(),
                )
            }
            ValueType::Annotation => {
                debug_assert!(value_arg == 0);
                let offset = &mut 1;
                let result = EncodedValue::Annotation(source.gread_with(offset, dex)?);
                size_read += *offset;
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
            _ => {
                return Err(scroll::Error::Custom("Invalid Encoded Value Type".to_string()).into());
            }
        };
        Ok((value, size_read))
    }
}

#[derive(Debug)]
pub struct EncodedArray {
    values: Vec<EncodedValue>,
}
