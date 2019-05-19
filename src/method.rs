use scroll::ctx;
use scroll::Pread;
use scroll::Uleb128;

use crate::cache::Ref;
use crate::code::CodeItem;
use crate::encoded_item::EncodedItem;
use crate::encoded_item::EncodedItemArray;
use crate::jtype::Type;
use crate::jtype::TypeId;
use crate::string::JString;
use crate::string::StringId;
use crate::uint;
use crate::ushort;
use crate::ubyte;

#[derive(Debug)]
pub struct Method {
    class_id: Type,
    name: Ref<JString>,
    access_flags: u64,
    params: Option<Vec<Type>>,
    shorty: Ref<JString>,
    return_type: Type,
    code: Option<CodeItem>,
}

pub type ProtoId = u64;

#[derive(Pread)]
pub(crate) struct ProtoIdItem {
    shorty: StringId,
    return_type: TypeId,
    params_off: uint,
}

impl ProtoIdItem {
    pub(crate) fn try_from_dex<S: AsRef<[ubyte]>>(
        dex: &super::Dex<S>,
        offset: u64,
    ) -> super::Result<Self> {
        let source = dex.source.as_ref();
        Ok(source.pread_with(offset as usize, dex.get_endian())?)
    }
}

impl Method {
    pub(crate) fn try_from_dex<S: AsRef<[ubyte]>>(
        dex: &super::Dex<S>,
        encoded_method: &EncodedMethod,
    ) -> super::Result<Method> {
        let source = dex.source.as_ref();
        let method_item = dex.get_method_item(encoded_method.method_id)?;
        let proto_item = dex.get_proto_item(u64::from(method_item.proto_id))?;
        let shorty = dex.get_string(proto_item.shorty)?;
        let return_type = dex.get_type(proto_item.return_type)?;
        let params = if proto_item.params_off != 0 {
            let mut offset = proto_item.params_off as usize;
            let offset = &mut offset;
            let endian = dex.get_endian();
            let len = source.gread_with::<uint>(offset, endian)?;
            let mut types: Vec<Type> = Vec::with_capacity(len as usize);
            for _ in 0..len {
                let type_id: ushort = source.gread_with(offset, endian)?;
                types.push(dex.get_type(uint::from(type_id))?);
            }
            Some(types)
        } else {
            None
        };
        let code = if encoded_method.code_offset > 0 {
            Some(dex.get_code_item(encoded_method.code_offset)?)
        } else {
            None
        };
        Ok(Self {
            name: dex.get_string(method_item.name_id)?,
            class_id: dex.get_type(uint::from(method_item.class_id))?,
            access_flags: encoded_method.access_flags,
            shorty,
            return_type,
            params,
            code,
        })
    }
}

#[derive(Pread, Debug)]
pub(crate) struct MethodIdItem {
    class_id: ushort,
    proto_id: ushort,
    name_id: StringId,
}

impl MethodIdItem {
    pub(crate) fn try_from_dex<S: AsRef<[ubyte]>>(
        dex: &super::Dex<S>,
        offset: u64,
    ) -> super::Result<Self> {
        let source = dex.source.as_ref();
        Ok(source.pread_with(offset as usize, dex.get_endian())?)
    }
}

pub type MethodId = u64;

#[derive(Debug)]
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

    fn try_from_ctx(source: &'a [ubyte], prev_id: u64) -> super::Result<(Self, Self::Size)> {
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
