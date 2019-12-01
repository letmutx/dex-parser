//! Dex `Method` and supporting structures
use getset::{CopyGetters, Getters};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use scroll::{ctx, Pread, Uleb128};

use crate::{
    annotation::{AnnotationSetItem, AnnotationSetRefList},
    code::CodeItem,
    encoded_item::{EncodedItem, EncodedItemArray},
    error::Error,
    field::FieldId,
    jtype::{Type, TypeId},
    string::{DexString, StringId},
    uint, ulong, ushort, utils,
};

bitflags! {
    /// Access flags of a `Dex` Method
    pub struct AccessFlags: ulong {
        const PUBLIC = 0x1;
        const PRIVATE = 0x2;
        const PROTECTED = 0x4;
        const STATIC = 0x8;
        const FINAL = 0x10;
        const SYNCHRONIZED = 0x20;
        const BRIDGE = 0x40;
        const VARARGS = 0x80;
        const NATIVE = 0x100;
        const ABSTRACT = 0x400;
        const STRICT = 0x800;
        const SYNTHETIC = 0x1000;
        const CONSTRUCTOR = 0x10000;
        const DECLARED_SYNCHRONIZED = 0x20000;
    }
}

/// Represents a `Class` method.
#[derive(Debug, Getters, CopyGetters)]
pub struct Method {
    /// Parent class of the method.
    #[get = "pub"]
    class: Type,
    /// Name of the method.
    #[get = "pub"]
    name: DexString,
    /// Access flags of the method.
    #[get_copy = "pub"]
    access_flags: AccessFlags,
    /// Types of the parameters of the method.
    #[get = "pub"]
    params: Vec<Type>,
    /// Shorty descriptor of the method, as described
    /// [here](https://source.android.com/devices/tech/dalvik/dex-format#shortydescriptor)
    #[get = "pub"]
    shorty: DexString,
    /// Return type of the method.
    #[get = "pub"]
    return_type: Type,
    /// Code and DebugInfo of the method.
    code: Option<CodeItem>,
    /// Annotations of the method.
    #[get = "pub"]
    annotations: AnnotationSetItem,
    /// Annotations of the params.
    #[get = "pub"]
    param_annotations: AnnotationSetRefList,
}

impl Method {
    gen_is_flag_set!(is_public, PUBLIC);
    gen_is_flag_set!(is_private, PRIVATE);
    gen_is_flag_set!(is_protected, PROTECTED);
    gen_is_flag_set!(is_static, STATIC);
    gen_is_flag_set!(is_final, FINAL);
    gen_is_flag_set!(is_synchronized, SYNCHRONIZED);
    gen_is_flag_set!(is_bridge, BRIDGE);
    gen_is_flag_set!(is_varargs, VARARGS);
    gen_is_flag_set!(is_native, NATIVE);
    gen_is_flag_set!(is_abstract, ABSTRACT);
    gen_is_flag_set!(is_strict, STRICT);
    gen_is_flag_set!(is_synthetic, SYNTHETIC);
    gen_is_flag_set!(is_constructor, CONSTRUCTOR);
    gen_is_flag_set!(is_declared_synchronized, DECLARED_SYNCHRONIZED);

    /// Code and DebugInfo of the method.
    pub fn code(&self) -> Option<&CodeItem> {
        self.code.as_ref()
    }
}

/// Index into the `ProtoId`s list.
pub type ProtoId = ulong;

/// Method Prototypes.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#proto-id-item)
#[derive(Pread, Debug, CopyGetters, PartialEq)]
#[get_copy = "pub"]
pub struct ProtoIdItem {
    /// Index into the string_ids list for the short-form descriptor string of this prototype
    shorty: StringId,
    /// Index into the type_ids list for the return type of this prototype.
    return_type: TypeId,
    /// Offset from the start of the file to the list of parameter types for this prototype, or `0`
    /// if this prototype has no params. The data at the location should be a list of types.
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
        annotations: AnnotationSetItem,
        param_annotations: AnnotationSetRefList,
    ) -> super::Result<Method> {
        debug!(target: "method", "encoded method: {:?}", encoded_method);
        let source = &dex.source;
        let method_item = dex.get_method_item(encoded_method.method_id)?;
        let name = dex.get_string(method_item.name_idx)?;
        debug!(target: "method", "name: {}, method id item: {:?}", name, method_item);
        let proto_item = dex.get_proto_item(ProtoId::from(method_item.proto_idx))?;
        debug!(target: "method", "method proto_item: {:?}", proto_item);
        let shorty = dex.get_string(proto_item.shorty)?;
        let return_type = dex.get_type(proto_item.return_type)?;
        let params = if proto_item.params_off != 0 {
            if !dex.is_offset_in_data_section(proto_item.params_off) {
                return Err(Error::BadOffset(
                    proto_item.params_off as usize,
                    format!(
                        "Params offset not in data section for proto_item: {:?}",
                        proto_item
                    ),
                ));
            }
            let offset = &mut (proto_item.params_off as usize);
            let endian = dex.get_endian();
            let len = source.gread_with::<uint>(offset, endian)?;
            let type_ids: Vec<ushort> = try_gread_vec_with!(source, offset, len, endian);
            utils::get_types(dex, &type_ids)?
        } else {
            Default::default()
        };
        debug!(target: "method", "code item offset: {}", encoded_method.code_offset);
        let code = dex.get_code_item(encoded_method.code_offset)?;
        Ok(Self {
            name,
            class: dex.get_type(TypeId::from(method_item.class_idx))?,
            access_flags: AccessFlags::from_bits(encoded_method.access_flags).ok_or_else(|| {
                Error::InvalidId(format!(
                    "Invalid access flags for method {}",
                    method_item.name_idx
                ))
            })?,
            shorty,
            return_type,
            params,
            code,
            annotations,
            param_annotations,
        })
    }
}

/// Method identifier.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#method-id-item)
#[derive(Pread, Debug, CopyGetters, PartialEq)]
#[get_copy = "pub"]
pub struct MethodIdItem {
    /// Index into the `TypeId`s list for the definer of this method.
    class_idx: ushort,
    /// Index into the `ProtoId`s list for the prototype of this method.
    proto_idx: ushort,
    /// Index into the `StringId`s list for the name of this method.
    name_idx: StringId,
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

/// Index into the `MethodId`s list.
pub type MethodId = ulong;

/// Contains a `MethodId` along with its access flags and code.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#encoded-method)
#[derive(Debug, Getters, CopyGetters)]
pub struct EncodedMethod {
    /// Index into the `MethodId`s list for the identity of this method represented as
    /// a difference from the index of previous element in the list.
    #[get_copy = "pub(crate)"]
    pub(crate) method_id: MethodId,
    /// Access flags for this method.
    #[get = "pub"]
    access_flags: ulong,
    /// Offset from the start of the file to the code structure for this method, or `0` if this
    /// method is either abstract or native.  The format of the data is specified by `CodeItem`.
    #[get = "pub"]
    code_offset: ulong,
}

impl EncodedItem for EncodedMethod {
    fn id(&self) -> ulong {
        self.method_id
    }
}

/// List of `EncodedMethod`s
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
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#method-handle-type-codes)
#[derive(FromPrimitive, Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldOrMethodId {
    Field(FieldId),
    Method(MethodId),
}

/// A method handle.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#method-handle-item)
#[derive(Debug, CopyGetters, PartialEq)]
#[get_copy = "pub"]
pub struct MethodHandleItem {
    ///  The type of this MethodHandleItem.
    handle_type: MethodHandleType,
    /// `FieldId` or `MethodId`  depending on whether the method handle type is an accessor or
    /// a method invoker
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
