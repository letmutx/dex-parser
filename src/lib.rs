extern crate num_derive;
#[macro_use]
extern crate scroll_derive;

use std::clone::Clone;
use std::fs::File;

use memmap::{Mmap, MmapOptions};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use scroll::{self, ctx, Pread};

use cache::Ref;
use class::{Class, ClassDataItem, ClassDefItemIter, ClassId};
use jtype::{Type, TypeId};
use source::Source;
use string::{JString, StringCache};

use crate::code::CodeItem;
use crate::field::EncodedField;
use crate::field::Field;
use crate::field::FieldId;
use crate::field::FieldIdItem;
use crate::method::EncodedMethod;
use crate::method::Method;
use crate::method::MethodId;
use crate::method::MethodIdItem;
use crate::method::ProtoId;
use crate::method::ProtoIdItem;

#[macro_use]
mod utils;
mod cache;
mod class;
mod code;
mod encoded_item;
mod encoded_value;
mod error;
mod field;
mod jtype;
mod method;
mod source;
mod string;

const NO_INDEX: uint = 0xffff_ffff;

#[allow(non_camel_case_types)]
pub type uint = u32;
#[allow(non_camel_case_types)]
pub type int = i32;
#[allow(non_camel_case_types)]
pub type ushort = u16;
#[allow(non_camel_case_types)]
pub type short = i16;
#[allow(non_camel_case_types)]
pub type ubyte = u8;
#[allow(non_camel_case_types)]
pub type ulong = u64;
#[allow(non_camel_case_types)]
pub type long = i64;

type Result<T> = std::result::Result<T, error::Error>;

// ref. https://source.android.com/devices/tech/dalvik/dex-format

#[derive(Debug, Pread)]
struct Header {
    magic: [ubyte; 8],
    checksum: uint,
    signature: [ubyte; 20],
    file_size: uint,
    header_size: uint,
    endian_tag: [ubyte; 4],
    link_size: uint,
    link_off: uint,
    map_off: uint,
    string_ids_size: uint,
    string_ids_off: uint,
    type_ids_size: uint,
    type_ids_off: uint,
    proto_ids_size: uint,
    proto_ids_off: uint,
    field_ids_size: uint,
    field_ids_off: uint,
    method_ids_size: uint,
    method_ids_off: uint,
    class_defs_size: uint,
    class_defs_off: uint,
    data_size: uint,
    data_off: uint,
}

pub struct Dex<T> {
    source: Source<T>,
    string_cache: StringCache<T>,
    inner: DexInner,
}

#[derive(Debug)]
struct DexInner {
    header: Header,
    map_list: MapList,
    endian: Endian,
}

impl DexInner {
    fn get_endian(&self) -> Endian {
        self.endian
    }

    fn strings_offset(&self) -> uint {
        self.header.string_ids_off
    }

    fn strings_len(&self) -> uint {
        self.header.string_ids_size
    }

    fn field_ids_len(&self) -> uint {
        self.header.field_ids_size
    }

    fn field_ids_offset(&self) -> uint {
        self.header.field_ids_off
    }

    fn class_defs_offset(&self) -> uint {
        self.header.class_defs_off
    }

    fn class_defs_len(&self) -> uint {
        self.header.class_defs_size
    }

    fn method_ids_offset(&self) -> uint {
        self.header.method_ids_off
    }

    fn method_ids_len(&self) -> uint {
        self.header.method_ids_size
    }

    fn proto_ids_offset(&self) -> uint {
        self.header.proto_ids_off
    }

    fn proto_ids_len(&self) -> uint {
        self.header.proto_ids_size
    }

    fn type_ids_offset(&self) -> uint {
        self.header.type_ids_off
    }

    fn type_ids_len(&self) -> uint {
        self.header.type_ids_size
    }
}

#[derive(Debug)]
struct MapList {
    map_items: Vec<MapItem>,
}

#[derive(FromPrimitive, Debug)]
pub enum ItemType {
    Header = 0x0,
    StringIdItem = 0x1,
    TypeIdItem = 0x2,
    ProtoIdItem = 0x3,
    FieldIdItem = 0x4,
    MethodIdItem = 0x5,
    ClassDefItem = 0x6,
    CallSiteIdItem = 0x7,
    MethodHandleItem = 0x8,
    MapList = 0x1000,
    TypeList = 0x1001,
    AnnotationSetRefList = 0x1002,
    AnnotationSetItem = 0x1003,
    ClassDataItem = 0x2000,
    CodeItem = 0x2001,
    StringDataItem = 0x2002,
    DebugInfoItem = 0x2003,
    AnnotationItem = 0x2004,
    EncodedArrayItem = 0x2005,
    AnnotationsDirectoryItem = 0x2006,
}

#[derive(Debug)]
struct MapItem {
    item_type: ItemType,
    size: uint,
    offset: uint,
}

impl<'a> ctx::TryFromCtx<'a, Endian> for MapItem {
    type Error = error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], endian: Endian) -> Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let item_type: ushort = source.gread_with(offset, endian)?;
        let item_type = ItemType::from_u16(item_type).ok_or(crate::error::Error::InvalidId(
            format!("Invalid item type in map_list: {}", item_type),
        ))?;
        let _: ushort = source.gread_with(offset, endian)?;
        let size: uint = source.gread_with(offset, endian)?;
        let item_offset: uint = source.gread_with(offset, endian)?;
        Ok((
            Self {
                item_type,
                size,
                offset: item_offset,
            },
            *offset,
        ))
    }
}

impl<'a> ctx::TryFromCtx<'a, Endian> for MapList {
    type Error = error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], endian: Endian) -> Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let size: uint = source.gread_with(offset, endian)?;
        Ok((
            Self {
                map_items: gread_vec_with!(source, &mut 0, size, endian),
            },
            *offset,
        ))
    }
}

impl<'a> ctx::TryFromCtx<'a, ()> for DexInner {
    type Error = error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], _: ()) -> Result<(Self, Self::Size)> {
        let endian_tag = &source[40..44];
        let endian = match (endian_tag[0], endian_tag[1], endian_tag[2], endian_tag[3]) {
            (0x12, 0x34, 0x56, 0x78) => scroll::BE,
            (0x78, 0x56, 0x34, 0x12) => scroll::LE,
            _ => return Err(error::Error::MalFormed("Bad endian tag".to_string())),
        };
        let header = source.pread_with::<Header>(0, endian)?;
        let map_list = source.pread_with(header.map_off as usize, endian)?;
        Ok((
            DexInner {
                header,
                map_list,
                endian,
            },
            0,
        ))
    }
}

pub type Endian = scroll::Endian;

pub struct DexBuilder;

impl DexBuilder {
    pub fn from_file(file: &File) -> Result<Dex<Mmap>> {
        let map = unsafe { MmapOptions::new().map(&file)? };
        let inner: DexInner = map.pread(0)?;
        let endian = inner.get_endian();
        let source = Source::new(map);
        let cache = StringCache::new(
            source.clone(),
            endian,
            inner.strings_offset(),
            inner.strings_len(),
            4096,
        );
        Ok(Dex {
            source: source.clone(),
            string_cache: cache,
            inner,
        })
    }
}

#[derive(Debug)]
pub struct MethodHandleItem;

impl<T> Dex<T>
where
    T: AsRef<[u8]>,
{
    fn get_source_file(&self, file_id: string::StringId) -> Result<Option<Ref<JString>>> {
        if file_id == NO_INDEX {
            Ok(None)
        } else {
            Ok(Some(self.get_string(file_id)?))
        }
    }

    pub fn get_string(&self, id: string::StringId) -> Result<Ref<JString>> {
        self.string_cache.get(id)
    }

    pub fn get_type(&self, type_id: TypeId) -> Result<Type> {
        let max_offset = self.inner.type_ids_offset() + (self.inner.type_ids_len() - 1) * 4;
        let offset = self.inner.type_ids_offset() + type_id * 4;
        if offset > max_offset {
            return Err(crate::error::Error::InvalidId(format!(
                "Invalid type id: {}",
                type_id
            )));
        }
        let string_id = self
            .source
            .as_ref()
            .pread_with(offset as usize, self.get_endian())?;
        self.get_string(string_id)
            .map(|string| Type(type_id, string))
    }

    pub fn get_class(&self, class_id: ClassId) -> Option<Result<Class>> {
        // TODO: can do binary search
        self.classes()
            .filter(|c| match c {
                Ok(c) => c.id == class_id,
                Err(_) => false,
            })
            .take(1)
            .next()
    }

    pub fn get_class_by_name(&self, jtype: &Type) -> Option<Result<Class>> {
        // TODO: can do binary search
        self.classes()
            .filter(|class| match class {
                Ok(c) => c.get_type() == *jtype,
                Err(_) => false,
            })
            .take(1)
            .next()
    }

    fn get_interfaces(&self, offset: uint) -> Result<Option<Vec<Type>>> {
        let mut offset = offset as usize;
        if offset == 0 {
            return Ok(None);
        }
        let source = self.source.as_ref();
        let endian = self.get_endian();
        let len = source.gread_with::<uint>(&mut offset, endian)?;
        let type_ids: Vec<ushort> = gread_vec_with!(source, &mut offset, len, endian);
        Ok(Some(utils::get_types(self, &type_ids)?))
    }

    fn get_field_item(&self, field_id: FieldId) -> Result<FieldIdItem> {
        let offset = ulong::from(self.inner.field_ids_offset()) + field_id * 8;
        let max_offset = self.inner.field_ids_offset() + (self.inner.field_ids_len() - 1) * 8;
        let max_offset = ulong::from(max_offset);
        if offset > max_offset {
            return Err(error::Error::InvalidId(format!(
                "Invalid field id: {}",
                field_id
            )));
        }
        FieldIdItem::try_from_dex(self, offset)
    }

    fn get_proto_item(&self, proto_id: ProtoId) -> Result<ProtoIdItem> {
        let offset = ulong::from(self.inner.proto_ids_offset()) + proto_id * 12;
        let max_offset = ulong::from(self.inner.proto_ids_offset())
            + ulong::from((self.inner.proto_ids_len() - 1) * 12);
        if offset > max_offset {
            return Err(error::Error::InvalidId(format!(
                "Invalid proto id: {}",
                proto_id
            )));
        }
        ProtoIdItem::try_from_dex(self, offset)
    }

    fn get_method_item(&self, method_id: MethodId) -> Result<MethodIdItem> {
        let offset = ulong::from(self.inner.method_ids_offset()) + method_id * 8;
        let max_offset = self.inner.method_ids_offset() + (self.inner.method_ids_len() - 1) * 8;
        let max_offset = ulong::from(max_offset);
        if offset > max_offset {
            return Err(error::Error::InvalidId(format!(
                "Invalid method id: {}",
                method_id
            )));
        }
        MethodIdItem::try_from_dex(self, offset)
    }

    fn get_field(&self, encoded_field: &EncodedField) -> Result<Field> {
        Field::try_from_dex(self, encoded_field)
    }

    fn get_method(&self, encoded_method: &EncodedMethod) -> Result<Method> {
        Method::try_from_dex(self, encoded_method)
    }

    fn get_class_data(&self, offset: uint) -> Result<Option<ClassDataItem>> {
        if offset == 0 {
            return Ok(None);
        }
        Ok(Some(
            self.source.as_ref().pread_with(offset as usize, self)?,
        ))
    }

    fn get_method_handle_item(&self, _method_handle_id: u32) -> Result<MethodHandleItem> {
        unimplemented!()
    }

    pub fn get_endian(&self) -> Endian {
        self.inner.get_endian()
    }

    pub fn classes(&self) -> impl Iterator<Item = Result<Class>> + '_ {
        let defs_len = self.inner.class_defs_len();
        let defs_offset = self.inner.class_defs_offset();
        let source = self.source.clone();
        let endian = self.get_endian();
        ClassDefItemIter::new(source, defs_offset, defs_len, endian)
            .map(move |class_def_item| Class::try_from_dex(&self, &class_def_item?))
    }

    fn get_code_item(&self, code_off: ulong) -> Result<Option<CodeItem>> {
        if code_off == 0 {
            return Ok(None);
        }

        Ok(Some(self.source.pread_with(code_off as usize, self)?))
    }
}
