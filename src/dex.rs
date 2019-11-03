use std::fs::File;

use memmap::Mmap;
use memmap::MmapOptions;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use scroll::ctx;
use scroll::Pread;

use super::Result;
use crate::annotation::{
    AnnotationItem, AnnotationSetItem, AnnotationSetRefList, AnnotationsDirectoryItem,
};
use crate::cache::Ref;
use crate::class::Class;
use crate::class::ClassDataItem;
use crate::class::ClassDefItem;
use crate::class::ClassDefItemIter;
use crate::class::ClassId;
use crate::code::{CodeItem, DebugInfoItem};
use crate::error;
use crate::error::Error;
use crate::field::EncodedField;
use crate::field::Field;
use crate::field::FieldId;
use crate::field::FieldIdItem;
use crate::jtype::Type;
use crate::jtype::TypeId;
use crate::method::EncodedMethod;
use crate::method::Method;
use crate::method::MethodHandleItem;
use crate::method::MethodId;
use crate::method::MethodIdItem;
use crate::method::ProtoId;
use crate::method::ProtoIdItem;
use crate::source::Source;
use crate::string::JString;
use crate::string::StringCache;
use crate::string::StringId;
use crate::string::Strings;
use crate::ubyte;
use crate::uint;
use crate::ulong;
use crate::ushort;
use crate::utils;
use crate::Endian;
use crate::{ENDIAN_CONSTANT, NO_INDEX, REVERSE_ENDIAN_CONSTANT};
use std::path::Path;

/// Dex file header
#[derive(Debug, Pread)]
struct Header {
    /// Magic value that must appear at the beginning of the header section
    /// Contains dex\n<version>\0
    magic: [ubyte; 8],
    /// Adler32 checksum of the rest of the file (everything but magic and this field);
    /// Used to detect file corruption.
    checksum: uint,
    /// SHA-1 signature (hash) of the rest of the file (everything but magic, checksum, and this field);
    /// Used to uniquely identify files.
    signature: [ubyte; 20],
    /// Size of the entire file (including the header), in bytes.
    file_size: uint,
    /// Size of the header in bytes. Usually 0x70.
    header_size: uint,
    /// Endianness tag
    /// A value of 0x12345678 denotes little-endian, 0x78563412 denotes byte-swapped form.
    endian_tag: [ubyte; 4],
    /// Size of the link section, or 0 if this file isn't statically linked
    link_size: uint,
    /// Offset from the start of the file to the link section
    /// The offset, if non-zero, should be into the link_data section.
    link_off: uint,
    /// Offset from the start of the file to the map item.
    /// Must be non-zero and into the data section.
    map_off: uint,
    /// Count of strings in the string identifiers list.
    string_ids_size: uint,
    /// Offset from the start of the file to the string identifiers list
    /// The offset, if non-zero, should be to the start of the string_ids section.
    string_ids_off: uint,
    /// Count of elements in the type identifiers list, at most 65535.
    type_ids_size: uint,
    /// Offset from the start of the file to the type identifiers list
    /// The offset, if non-zero, should be to the start of the type_ids section.
    type_ids_off: uint,
    /// Count of elements in the prototype identifiers list, at most 65535.
    proto_ids_size: uint,
    /// Offset from the start of the file to the prototype identifiers list.
    /// The offset, if non-zero, should be to the start of the proto_ids section.
    proto_ids_off: uint,
    /// Count of elements in the field identifiers list
    field_ids_size: uint,
    /// Offset from the start of the file to the field identifiers list
    /// The offset, if non-zero, should be to the start of the field_ids section
    field_ids_off: uint,
    /// Count of elements in the method identifiers list
    method_ids_size: uint,
    /// Offset from the start of the file to the method identifiers list.
    /// The offset, if non-zero, should be to the start of the method_ids section.
    method_ids_off: uint,
    /// Count of elements in the class definitions list
    class_defs_size: uint,
    /// Offset from the start of the file to the class definitions list.
    /// The offset, if non-zero, should be to the start of the class_defs section.
    class_defs_off: uint,
    /// Size of data section in bytes. Must be an even multiple of sizeof(uint).
    data_size: uint,
    /// Offset from the start of the file to the start of the data section.
    data_off: uint,
}

/// Wrapper type for Dex
#[derive(Debug)]
pub(crate) struct DexInner {
    /// The header
    header: Header,
    map_list: MapList,
    endian: Endian,
}

impl DexInner {
    pub(crate) fn get_endian(&self) -> Endian {
        self.endian
    }

    pub(crate) fn strings_offset(&self) -> uint {
        self.header.string_ids_off
    }

    pub(crate) fn strings_len(&self) -> uint {
        self.header.string_ids_size
    }

    pub(crate) fn field_ids_len(&self) -> uint {
        self.header.field_ids_size
    }

    pub(crate) fn field_ids_offset(&self) -> uint {
        self.header.field_ids_off
    }

    pub(crate) fn class_defs_offset(&self) -> uint {
        self.header.class_defs_off
    }

    pub(crate) fn class_defs_len(&self) -> uint {
        self.header.class_defs_size
    }

    pub(crate) fn method_ids_offset(&self) -> uint {
        self.header.method_ids_off
    }

    pub(crate) fn method_ids_len(&self) -> uint {
        self.header.method_ids_size
    }

    pub(crate) fn proto_ids_offset(&self) -> uint {
        self.header.proto_ids_off
    }

    pub(crate) fn proto_ids_len(&self) -> uint {
        self.header.proto_ids_size
    }

    pub(crate) fn type_ids_offset(&self) -> uint {
        self.header.type_ids_off
    }

    pub(crate) fn type_ids_len(&self) -> uint {
        self.header.type_ids_size
    }

    fn method_handles_offset(&self) -> Option<uint> {
        self.map_list.get_offset(ItemType::MethodHandleItem)
    }

    fn method_handles_len(&self) -> Option<uint> {
        self.map_list.get_len(ItemType::MethodHandleItem)
    }
}

// TODO: this should be try_from_dex
impl<'a> ctx::TryFromCtx<'a, ()> for DexInner {
    type Error = error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], _: ()) -> Result<(Self, Self::Size)> {
        if source.len() <= 44 {
            return Err(Error::MalFormed("Invalid dex file".to_string()));
        }
        let endian_tag = &source[40..44];
        let endian = match (endian_tag[0], endian_tag[1], endian_tag[2], endian_tag[3]) {
            ENDIAN_CONSTANT => scroll::BE,
            REVERSE_ENDIAN_CONSTANT => scroll::LE,
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

/// List of the entire contents of a file, in order. A given type must appear at most
/// once in a map, entries must be ordered by initial offset and must not overlap.
#[derive(Debug)]
struct MapList {
    map_items: Vec<MapItem>,
}

impl<'a> ctx::TryFromCtx<'a, Endian> for MapList {
    type Error = error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], endian: Endian) -> Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let size: uint = source.gread_with(offset, endian)?;
        Ok((
            Self {
                map_items: try_gread_vec_with!(source, offset, size, endian),
            },
            *offset,
        ))
    }
}

impl MapList {
    fn get(&self, item_type: ItemType) -> Option<MapItem> {
        self.map_items
            .iter()
            .find(|map_item| map_item.item_type == item_type)
            .cloned()
    }

    fn get_offset(&self, item_type: ItemType) -> Option<uint> {
        self.get(item_type).map(|map_item| map_item.offset)
    }

    fn get_len(&self, item_type: ItemType) -> Option<uint> {
        self.get(item_type).map(|map_item| map_item.size)
    }
}

/// ItemType that appear in MapList
#[derive(FromPrimitive, Debug, Clone, Copy, Eq, PartialEq)]
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

///
#[derive(Debug, Clone, Copy)]
struct MapItem {
    /// Type of the current item
    item_type: ItemType,
    /// Count of the number of items to be found at the indicated offset
    size: uint,
    /// Offset from the start of the file to the current item type
    offset: uint,
}

impl<'a> ctx::TryFromCtx<'a, Endian> for MapItem {
    type Error = error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], endian: Endian) -> Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let item_type: ushort = source.gread_with(offset, endian)?;
        let item_type = ItemType::from_u16(item_type).ok_or_else(|| {
            Error::InvalidId(format!("Invalid item type in map_list: {}", item_type))
        })?;
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

/// Represents a Dex file
pub struct Dex<T> {
    /// Source from which this Dex file is loaded from.
    pub(crate) source: Source<T>,
    /// Items in string_ids section are cached here.
    pub(crate) string_cache: StringCache<T>,
    pub(crate) inner: DexInner,
}

impl<T> Dex<T>
where
    T: AsRef<[u8]>,
{
    pub(crate) fn get_source_file(&self, file_id: StringId) -> Result<Option<Ref<JString>>> {
        Ok(if file_id == NO_INDEX {
            None
        } else {
            Some(self.get_string(file_id)?)
        })
    }

    /// Returns a reference to the `JString` represented by the given id.
    pub fn get_string(&self, string_id: StringId) -> Result<Ref<JString>> {
        if self.inner.strings_len() <= string_id {
            return Err(Error::InvalidId(format!(
                "Invalid string id: {}",
                string_id
            )));
        }
        self.string_cache.get(string_id)
    }

    /// Returns the `Type` represented by the give type_id.
    pub fn get_type(&self, type_id: TypeId) -> Result<Type> {
        let max_offset = self.inner.type_ids_offset() + (self.inner.type_ids_len() - 1) * 4;
        let offset = self.inner.type_ids_offset() + type_id * 4;
        if offset > max_offset {
            return Err(Error::InvalidId(format!("Invalid type id: {}", type_id)));
        }
        let string_id = self
            .source
            .as_ref()
            .pread_with(offset as usize, self.get_endian())?;
        self.get_string(string_id).map(|type_descriptor| Type {
            id: type_id,
            type_descriptor,
        })
    }

    fn find_class<F: Fn(&ClassDefItem) -> Result<bool>>(
        &self,
        predicate: F,
    ) -> Result<Option<Class>> {
        for class_def in self.class_defs() {
            let class_def = class_def?;
            if predicate(&class_def)? {
                return Ok(Some(Class::try_from_dex(self, &class_def)?));
            }
        }
        Ok(None)
    }

    /// Returns the `Class` represented by the given class_id.
    pub fn get_class(&self, class_id: ClassId) -> Result<Option<Class>> {
        self.find_class(|class_def| Ok(class_def.class_idx == class_id))
    }

    /// Returns the `Class` represented by the given type.
    pub fn get_class_by_type(&self, jtype: &Type) -> Result<Option<Class>> {
        self.find_class(|class_def| {
            let class_type = self.get_type(class_def.class_idx)?;
            Ok(*jtype == class_type)
        })
    }

    pub(crate) fn get_interfaces(&self, offset: uint) -> Result<Option<Vec<Type>>> {
        let mut offset = offset as usize;
        if offset == 0 {
            return Ok(None);
        }
        let source = self.source.as_ref();
        let endian = self.get_endian();
        let len = source.gread_with::<uint>(&mut offset, endian)?;
        let offset = &mut offset;
        let type_ids: Vec<ushort> = try_gread_vec_with!(source, offset, len, endian);
        Ok(Some(utils::get_types(self, &type_ids)?))
    }

    pub(crate) fn get_field_item(&self, field_id: FieldId) -> Result<FieldIdItem> {
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

    pub(crate) fn get_proto_item(&self, proto_id: ProtoId) -> Result<ProtoIdItem> {
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

    pub(crate) fn get_method_item(&self, method_id: MethodId) -> Result<MethodIdItem> {
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

    /// Iterator over the strings
    pub fn strings(&self) -> impl Iterator<Item = Result<Ref<JString>>> {
        Strings::new(self.string_cache.clone(), self.inner.strings_len() as usize)
    }

    pub(crate) fn get_field(&self, encoded_field: &EncodedField) -> Result<Field> {
        Field::try_from_dex(self, encoded_field)
    }

    pub(crate) fn get_method(&self, encoded_method: &EncodedMethod) -> Result<Method> {
        Method::try_from_dex(self, encoded_method)
    }

    pub(crate) fn get_class_data(&self, offset: uint) -> Result<Option<ClassDataItem>> {
        if offset == 0 {
            return Ok(None);
        }
        Ok(Some(
            self.source.as_ref().pread_with(offset as usize, self)?,
        ))
    }

    pub(crate) fn get_method_handle_item(
        &self,
        method_handle_id: uint,
    ) -> Result<MethodHandleItem> {
        let err = || Error::InvalidId(format!("Invalid method handle id: {}", method_handle_id));
        let offset = self.inner.method_handles_offset().ok_or_else(err)?;
        let len = self.inner.method_handles_len().ok_or_else(err)?;
        let max_offset = offset + (len - 1) * 8;
        let offset = offset + method_handle_id * 8;
        if offset > max_offset {
            return Err(err());
        }
        self.source.gread_with(&mut (offset as usize), self)
    }

    pub(crate) fn get_endian(&self) -> Endian {
        self.inner.get_endian()
    }

    pub(crate) fn class_defs(&self) -> impl Iterator<Item = Result<ClassDefItem>> + '_ {
        let defs_len = self.inner.class_defs_len();
        let defs_offset = self.inner.class_defs_offset();
        let source = self.source.clone();
        let endian = self.get_endian();
        ClassDefItemIter::new(source, defs_offset, defs_len, endian)
    }

    /// Iterator over the classes
    pub fn classes(&self) -> impl Iterator<Item = Result<Class>> + '_ {
        self.class_defs()
            .map(move |class_def_item| Class::try_from_dex(&self, &class_def_item?))
    }

    pub(crate) fn get_code_item(&self, code_off: ulong) -> Result<Option<CodeItem>> {
        if code_off == 0 {
            return Ok(None);
        }

        Ok(Some(self.source.pread_with(code_off as usize, self)?))
    }

    pub(crate) fn get_annotation_item(&self, annotation_off: uint) -> Result<AnnotationItem> {
        Ok(self.source.pread_with(annotation_off as usize, self)?)
    }

    pub(crate) fn get_annotation_set_item(
        &self,
        annotation_set_item_off: uint,
    ) -> Result<AnnotationSetItem> {
        Ok(self
            .source
            .pread_with(annotation_set_item_off as usize, self)?)
    }

    pub(crate) fn get_annotation_set_ref_list(
        &self,
        annotation_set_ref_list_off: uint,
    ) -> Result<AnnotationSetRefList> {
        Ok(self
            .source
            .pread_with(annotation_set_ref_list_off as usize, self)?)
    }

    pub(crate) fn get_annotations_directory_item(
        &self,
        annotations_directory_item_off: uint,
    ) -> Result<AnnotationsDirectoryItem> {
        Ok(self
            .source
            .pread_with(annotations_directory_item_off as usize, self)?)
    }

    pub(crate) fn get_debug_info_item(&self, debug_info_off: uint) -> Result<DebugInfoItem> {
        Ok(self.source.pread_with(debug_info_off as usize, self)?)
    }
}

pub struct DexReader;

impl DexReader {
    /// Try to read a `Dex` from the given path, returns error if
    /// the file is not a dex or in case of I/O errors
    pub fn from_file<P: AsRef<Path>>(file: P) -> Result<Dex<Mmap>> {
        let map = unsafe { MmapOptions::new().map(&File::open(file.as_ref())?)? };
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
