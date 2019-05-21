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
use crate::ulong;
use crate::ushort;
use crate::utils;

#[derive(Debug)]
pub struct Method {
    class_id: Type,
    name: Ref<JString>,
    access_flags: ulong,
    params: Option<Vec<Type>>,
    shorty: Ref<JString>,
    return_type: Type,
    code: Option<CodeItem>,
}

pub type ProtoId = ulong;

#[derive(Pread)]
pub(crate) struct ProtoIdItem {
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
        let source = dex.source.as_ref();
        let method_item = dex.get_method_item(encoded_method.method_id)?;
        let proto_item = dex.get_proto_item(ulong::from(method_item.proto_id))?;
        let shorty = dex.get_string(proto_item.shorty)?;
        let return_type = dex.get_type(proto_item.return_type)?;
        let params = if proto_item.params_off != 0 {
            let offset = &mut (proto_item.params_off as usize);
            let endian = dex.get_endian();
            let len = source.gread_with::<uint>(offset, endian)?;
            let type_ids: Vec<ushort> = gread_vec_with!(source, offset, len, endian);
            Some(utils::get_types(dex, &type_ids)?)
        } else {
            None
        };
        let code = dex.get_code_item(encoded_method.code_offset)?;
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
    pub(crate) fn try_from_dex<S: AsRef<[u8]>>(
        dex: &super::Dex<S>,
        offset: ulong,
    ) -> super::Result<Self> {
        let source = dex.source.as_ref();
        Ok(source.pread_with(offset as usize, dex.get_endian())?)
    }
}

pub type MethodId = ulong;

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
    type Error = crate::error::Error;
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
