use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use scroll::ctx;
use scroll::Pread;
use scroll::Uleb128;

use crate::cache::Ref;
use crate::code::CodeItem;
use crate::encoded_item::EncodedItem;
use crate::encoded_item::EncodedItemArray;
use crate::error::Error;
use crate::field::FieldId;
use crate::jtype::Type;
use crate::jtype::TypeId;
use crate::string::JString;
use crate::string::StringId;
use crate::uint;
use crate::ulong;
use crate::ushort;
use crate::utils;
use crate::MethodAccessFlags;

// TODO: add accessor methods
/// Represents a class method.
#[derive(Debug)]
pub struct Method {
    /// Parent class of the method.
    class: Type,
    /// Name of the method.
    name: Ref<JString>,
    /// Access flags of the method.
    access_flags: MethodAccessFlags,
    /// Types of the parameters of the method.
    params: Option<Vec<Type>>,
    /// Shorty descriptor of the method. Conforms to
    /// https://source.android.com/devices/tech/dalvik/dex-format#shortydescriptor
    shorty: Ref<JString>,
    /// Return type of the method.
    return_type: Type,
    /// Code and DebugInfo of the method.
    code: Option<CodeItem>,
}

pub type ProtoId = ulong;

/// https://source.android.com/devices/tech/dalvik/dex-format#proto-id-item
#[derive(Pread, Debug)]
pub struct ProtoIdItem {
    shorty: StringId,
    return_type: TypeId,
    params_off: uint,
}

impl ProtoIdItem {
    pub(crate) fn try_from_dex<S: AsRef<[u8]>>(
        dex: &super::Dex<S>,
        offset: ulong,
    ) -> super::Result<Self> {
        let source = dex.source.as_ref();
        Ok(source.pread_with(offset as usize, dex.get_endian())?)
    }
}

impl Method {
    pub(crate) fn try_from_dex<S: AsRef<[u8]>>(
        dex: &super::Dex<S>,
        encoded_method: &EncodedMethod,
    ) -> super::Result<Method> {
        debug!(target: "method", "encoded method: {:?}", encoded_method);
        let source = &dex.source;
        let method_item = dex.get_method_item(encoded_method.method_id)?;
        let name = dex.get_string(method_item.name_id)?;
        debug!(target: "method", "name: {}, method id item: {:?}", name, method_item);
        let proto_item = dex.get_proto_item(ulong::from(method_item.proto_id))?;
        debug!(target: "method", "method proto_item: {:?}", proto_item);
        let shorty = dex.get_string(proto_item.shorty)?;
        let return_type = dex.get_type(proto_item.return_type)?;
        let params = if proto_item.params_off != 0 {
            let offset = &mut (proto_item.params_off as usize);
            let endian = dex.get_endian();
            let len = source.gread_with::<uint>(offset, endian)?;
            let type_ids: Vec<ushort> = try_gread_vec_with!(source, offset, len, endian);
            Some(utils::get_types(dex, &type_ids)?)
        } else {
            None
        };
        debug!(target: "method", "code item offset: {}", encoded_method.code_offset);
        let code = dex.get_code_item(encoded_method.code_offset)?;
        Ok(Self {
            name,
            class: dex.get_type(uint::from(method_item.class_id))?,
            access_flags: MethodAccessFlags::from_bits(encoded_method.access_flags).ok_or_else(
                || {
                    Error::InvalidId(format!(
                        "Invalid access flags for method {}",
                        method_item.name_id
                    ))
                },
            )?,
            shorty,
            return_type,
            params,
            code,
        })
    }
}

/// https://source.android.com/devices/tech/dalvik/dex-format#method-id-item
#[derive(Pread, Debug)]
pub struct MethodIdItem {
    class_id: ushort,
    proto_id: ushort,
    name_id: StringId,
}

impl MethodIdItem {
    pub(crate) fn try_from_dex<S: AsRef<[u8]>>(
        dex: &super::Dex<S>,
        offset: ulong,
    ) -> super::Result<Self> {
        let source = &dex.source;
        Ok(source.pread_with(offset as usize, dex.get_endian())?)
    }
}

pub type MethodId = ulong;

/// https://source.android.com/devices/tech/dalvik/dex-format#encoded-method
#[derive(Debug)]
pub(crate) struct EncodedMethod {
    pub(crate) method_id: MethodId,
    access_flags: ulong,
    code_offset: ulong,
}

impl EncodedItem for EncodedMethod {
    fn get_id(&self) -> ulong {
        self.method_id
    }
}

pub(crate) type EncodedMethodArray = EncodedItemArray<EncodedMethod>;

impl<'a> ctx::TryFromCtx<'a, ulong> for EncodedMethod {
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], prev_id: ulong) -> super::Result<(Self, Self::Size)> {
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

/// Type of the method handle.
/// https://source.android.com/devices/tech/dalvik/dex-format#method-handle-type-codes
#[derive(FromPrimitive, Debug)]
pub enum MethodHandleType {
    StaticPut = 0x00,
    StaticGet = 0x01,
    InstancePut = 0x02,
    InstanceGet = 0x03,
    InvokeStatic = 0x04,
    InvokeInstance = 0x05,
    InvokeConstructor = 0x06,
    InvokeDirect = 0x07,
    InvokeInterface = 0x08,
}

#[derive(Debug)]
pub enum FieldOrMethodId {
    Field(FieldId),
    Method(MethodId),
}

/// https://source.android.com/devices/tech/dalvik/dex-format#method-handle-item
#[derive(Debug)]
pub struct MethodHandleItem {
    handle_type: MethodHandleType,
    id: FieldOrMethodId,
}

impl<'a, S: AsRef<[u8]>> ctx::TryFromCtx<'a, &super::Dex<S>> for MethodHandleItem {
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], dex: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let endian = dex.get_endian();
        let offset = &mut 0;
        let handle_type: ushort = source.gread_with(offset, endian)?;
        let handle_type = MethodHandleType::from_u16(handle_type)
            .ok_or_else(|| Error::InvalidId(format!("Invalid handle type {}", handle_type)))?;
        let _: ushort = source.gread_with(offset, endian)?;
        let id: ushort = source.gread_with(offset, endian)?;
        let _: ushort = source.gread_with(offset, endian)?;
        let id = match handle_type {
            MethodHandleType::StaticPut
            | MethodHandleType::StaticGet
            | MethodHandleType::InstancePut
            | MethodHandleType::InstanceGet => FieldOrMethodId::Field(FieldId::from(id)),
            _ => FieldOrMethodId::Method(MethodId::from(id)),
        };

        Ok((Self { handle_type, id }, *offset))
    }
}
