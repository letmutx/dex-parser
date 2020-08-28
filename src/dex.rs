use std::{fs::File, io::BufReader, ops::Range};

use adler32;
use getset::{CopyGetters, Getters};
use memmap::{Mmap, MmapOptions};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use scroll::{ctx, Pread};

use super::Result;
use crate::{
    annotation::{
        AnnotationItem, AnnotationSetItem, AnnotationSetRefList, AnnotationsDirectoryItem,
    },
    class::{Class, ClassDataItem, ClassDefItem, ClassDefItemIter},
    code::{CodeItem, DebugInfoItem},
    encoded_value::{EncodedArray, EncodedValue},
    error::{self, Error},
    field::{EncodedField, Field, FieldId, FieldIdItem},
    jtype::{Type, TypeId},
    method::{
        EncodedMethod, Method, MethodHandleId, MethodHandleItem, MethodId, MethodIdItem, ProtoId,
        ProtoIdItem,
    },
    search::Section,
    source::Source,
    string::{DexString, StringId, Strings, StringsIter},
    ubyte, uint, ulong, ushort, utils, Endian, ENDIAN_CONSTANT, NO_INDEX, REVERSE_ENDIAN_CONSTANT,
};
use std::path::Path;

/// Dex file header
#[derive(Debug, Pread, CopyGetters)]
#[get_copy = "pub"]
pub struct Header {
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

impl Header {
    fn data_section(&self) -> Range<uint> {
        (self.data_off..self.data_off + self.data_size)
    }
}

/// Wrapper type for Dex
#[derive(Debug, Getters, CopyGetters)]
pub(crate) struct DexInner {
    /// The header
    #[get = "pub"]
    header: Header,
    /// Contents of the map_list section
    #[get = "pub"]
    map_list: MapList,
    #[get_copy = "pub"]
    endian: Endian,
}

impl DexInner {
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

    fn data_section(&self) -> Range<uint> {
        self.header.data_section()
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
            debug!("malformed dex: size < minimum header size");
            return Err(Error::MalFormed("Invalid dex file".to_string()));
        }
        let endian_tag = &source[40..44];
        let endian = match (endian_tag[0], endian_tag[1], endian_tag[2], endian_tag[3]) {
            ENDIAN_CONSTANT => scroll::BE,
            REVERSE_ENDIAN_CONSTANT => scroll::LE,
            _ => return Err(error::Error::MalFormed("Bad endian tag".to_string())),
        };
        let header = source.pread_with::<Header>(0, endian)?;
        if !header.data_section().contains(&header.map_off) {
            return Err(error::Error::BadOffset(
                header.map_off as usize,
                "map_list not in data section".to_string(),
            ));
        }
        let found = header.checksum();
        let computed = adler32::adler32(BufReader::new(&source[12..]))?;
        if computed != found {
            return Err(Error::MalFormed(format!(
                "File corrupted, adler32 checksum doesn't match: computed: {}, found: {}",
                computed, found
            )));
        }

        let map_list = source.pread_with(header.map_off as usize, endian)?;
        debug!(target: "initialization", "header: {:?}, endian-ness: {:?}", header, endian);
        debug!(target: "initialization", "map_list: {:?}", map_list);
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
pub struct MapList {
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
    /// Returns the `MapItem` corresponding to the `ItemType`.
    pub fn get(&self, item_type: ItemType) -> Option<MapItem> {
        self.map_items
            .iter()
            .find(|map_item| map_item.item_type == item_type)
            .cloned()
    }

    /// Returns the offset of the item corresponding to the `ItemType`.
    pub fn get_offset(&self, item_type: ItemType) -> Option<uint> {
        self.get(item_type).map(|map_item| map_item.offset)
    }

    /// Returns the length of the item corresponding to the `ItemType`.
    pub fn get_len(&self, item_type: ItemType) -> Option<uint> {
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

/// Single item of the MapList.
#[derive(Debug, Clone, Copy, CopyGetters)]
#[get_copy = "pub"]
pub struct MapItem {
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
    pub(crate) strings: Strings<T>,
    pub(crate) inner: DexInner,
}

impl<T> Dex<T>
where
    T: AsRef<[u8]>,
{
    /// The Header section
    pub fn header(&self) -> &Header {
        self.inner.header()
    }

    pub fn map_list(&self) -> &MapList {
        &self.inner.map_list
    }

    pub(crate) fn is_offset_in_data_section(&self, offset: uint) -> bool {
        self.inner.data_section().contains(&offset)
    }

    /// Source file name in which a class is defined.
    pub fn get_source_file(&self, file_id: StringId) -> Result<Option<DexString>> {
        Ok(if file_id == NO_INDEX {
            None
        } else {
            Some(self.get_string(file_id)?)
        })
    }

    /// Returns a reference to the `DexString` represented by the given id.
    pub fn get_string(&self, string_id: StringId) -> Result<DexString> {
        if self.inner.strings_len() <= string_id {
            return Err(Error::InvalidId(format!(
                "Invalid string id: {}",
                string_id
            )));
        }
        self.strings.get(string_id)
    }

    /// Returns the `Type` corresponding to the descriptor.
    pub fn get_type_from_descriptor(&self, descriptor: &str) -> Result<Option<Type>> {
        if let Some(string_id) = self.strings.get_id(descriptor)? {
            if let Some(type_id) = self.get_type_id(string_id)? {
                return Ok(Some(self.get_type(type_id)?));
            }
        }
        Ok(None)
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

    pub(crate) fn get_type_id(&self, string_id: StringId) -> Result<Option<TypeId>> {
        let types_section = self.type_ids_section();
        Ok(types_section
            .binary_search(
                &string_id,
                self.get_endian(),
                |value: &StringId, element| Ok((element).cmp(value)),
            )?
            .map(|s| s as TypeId))
    }

    pub(crate) fn type_ids_section(&self) -> Section {
        let type_ids_offset = self.inner.type_ids_offset() as usize;
        let (start, end) = (
            type_ids_offset,
            type_ids_offset + self.inner.type_ids_len() as usize * 4,
        );
        let type_ids_section = &self.source[start..end];
        Section::new(type_ids_section)
    }

    #[allow(unused)]
    pub(crate) fn class_defs_section(&self) -> Section {
        let class_defs_offset = self.inner.class_defs_offset() as usize;
        let (start, end) = (
            class_defs_offset,
            class_defs_offset + self.inner.class_defs_len() as usize * 32,
        );
        let class_defs_section = &self.source[start..end];
        Section::new(class_defs_section)
    }

    pub(crate) fn find_class_by_type(&self, type_id: TypeId) -> Result<Option<Class>> {
        for class_def in self.class_defs() {
            let class_def = class_def?;
            if class_def.class_idx == type_id {
                return Ok(Some(Class::try_from_dex(self, &class_def)?));
            }
        }
        Ok(None)
    }

    /// Finds `Class` by the given class name. The name should be in smali format.
    /// This method uses binary search to find the class definition using the property
    /// that the strings, type ids and class defs sections are in sorted.
    pub fn find_class_by_name(&self, type_descriptor: &str) -> Result<Option<Class>> {
        let string_id = self.strings.get_id(type_descriptor)?;
        if string_id.is_none() {
            debug!(target: "find-class-by-name", "class name: {} not found in strings", type_descriptor);
            return Ok(None);
        }
        let type_id = self.get_type_id(string_id.unwrap())?;
        if type_id.is_none() {
            debug!(target: "find-class-by-name", "no type id found for string id: {}", string_id.unwrap());
            return Ok(None);
        }
        self.find_class_by_type(type_id.unwrap())
    }

    /// Returns the list of types which represent the interfaces of a class.
    pub fn get_interfaces(&self, offset: uint) -> Result<Vec<Type>> {
        debug!(target: "interfaces", "interfaces offset: {}", offset);
        if offset == 0 {
            return Ok(Default::default());
        }
        if !self.is_offset_in_data_section(offset) {
            return Err(Error::BadOffset(
                offset as usize,
                "Interfaces offset not in data section".to_string(),
            ));
        }
        let mut offset = offset as usize;
        let source = &self.source;
        let endian = self.get_endian();
        let len = source.gread_with::<uint>(&mut offset, endian)?;
        debug!(target: "interfaces", "interfaces length: {}", len);
        let offset = &mut offset;
        let type_ids: Vec<ushort> = try_gread_vec_with!(source, offset, len, endian);
        utils::get_types(self, &type_ids)
    }

    /// Returns the `FieldIdItem` represented by a `FieldId`.
    pub fn get_field_item(&self, field_id: FieldId) -> Result<FieldIdItem> {
        let offset = ulong::from(self.inner.field_ids_offset()) + field_id * 8;
        let max_offset = self.inner.field_ids_offset() + (self.inner.field_ids_len() - 1) * 8;
        let max_offset = ulong::from(max_offset);
        debug!(target: "field-id-item", "current offset: {}, min_offset: {}, max_offset: {}",
                offset, self.inner.field_ids_offset(), max_offset);
        if offset > max_offset {
            return Err(error::Error::InvalidId(format!(
                "Invalid field id: {}",
                field_id
            )));
        }
        FieldIdItem::try_from_dex(self, offset)
    }

    /// Returns the `ProtoIdItem` represented by `ProtoId`.
    pub fn get_proto_item(&self, proto_id: ProtoId) -> Result<ProtoIdItem> {
        let offset = ulong::from(self.inner.proto_ids_offset()) + proto_id * 12;
        let max_offset = ulong::from(self.inner.proto_ids_offset())
            + ulong::from((self.inner.proto_ids_len() - 1) * 12);
        debug!(target: "proto-item", "proto item current offset: {}, min_offset: {}, max_offset: {}",
            offset, self.inner.proto_ids_offset(), max_offset);
        if offset > max_offset {
            return Err(error::Error::InvalidId(format!(
                "Invalid proto id: {}",
                proto_id
            )));
        }
        ProtoIdItem::try_from_dex(self, offset)
    }

    /// Returns the `MethodIdItem` represented by `MethodId`.
    pub fn get_method_item(&self, method_id: MethodId) -> Result<MethodIdItem> {
        let offset = ulong::from(self.inner.method_ids_offset()) + method_id * 8;
        let max_offset = self.inner.method_ids_offset() + (self.inner.method_ids_len() - 1) * 8;
        let max_offset = ulong::from(max_offset);
        debug!(target: "method-item", "method item current offset: {}, min_offset: {}, max_offset: {}",
            offset, self.inner.method_ids_offset(), max_offset);
        if offset > max_offset {
            return Err(error::Error::InvalidId(format!(
                "Invalid method id: {}",
                method_id
            )));
        }
        MethodIdItem::try_from_dex(self, offset)
    }

    /// Iterator over the strings
    pub fn strings(&self) -> impl Iterator<Item = Result<DexString>> {
        StringsIter::new(self.strings.clone(), self.inner.strings_len() as usize)
    }

    /// Returns a `Field` given its component items.
    pub fn get_field(
        &self,
        encoded_field: &EncodedField,
        initial_value: Option<EncodedValue>,
        annotations: AnnotationSetItem,
    ) -> Result<Field> {
        Field::try_from_dex(self, encoded_field, initial_value, annotations)
    }

    /// Returns a `Method` given its component items.
    pub fn get_method(
        &self,
        encoded_method: &EncodedMethod,
        method_annotations: AnnotationSetItem,
        parameter_annotations: AnnotationSetRefList,
    ) -> Result<Method> {
        Method::try_from_dex(
            self,
            encoded_method,
            method_annotations,
            parameter_annotations,
        )
    }

    /// Returns the `ClassDataItem` at the given offset.
    pub fn get_class_data(&self, offset: uint) -> Result<Option<ClassDataItem>> {
        debug!(target: "class-data", "class data offset: {}", offset);
        if offset == 0 {
            return Ok(None);
        }
        if !self.is_offset_in_data_section(offset) {
            return Err(Error::BadOffset(
                offset as usize,
                "ClassData offset not in data section".to_string(),
            ));
        }
        Ok(Some(self.source.pread_with(offset as usize, self)?))
    }

    /// Returns the `MethodHandleItem` represented by the `MethodHandleId`.
    pub fn get_method_handle_item(
        &self,
        method_handle_id: MethodHandleId,
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

    /// Returns the endianness in the header section.
    pub fn get_endian(&self) -> Endian {
        self.inner.endian()
    }

    /// Iterator over the class_defs section.
    pub fn class_defs(&self) -> impl Iterator<Item = Result<ClassDefItem>> + '_ {
        let defs_len = self.inner.class_defs_len();
        let defs_offset = self.inner.class_defs_offset();
        let source = self.source.clone();
        let endian = self.get_endian();
        ClassDefItemIter::new(source, defs_offset, defs_len, endian)
    }

    /// Iterator over the type_ids section.
    pub fn types(&self) -> impl Iterator<Item = Result<Type>> + '_ {
        let type_ids_len = self.inner.type_ids_len();
        (0..type_ids_len).map(move |type_id| self.get_type(type_id))
    }

    /// Iterator over the proto_ids section.
    pub fn proto_ids(&self) -> impl Iterator<Item = Result<ProtoIdItem>> + '_ {
        let proto_ids_len = self.inner.proto_ids_len();
        (0..proto_ids_len).map(move |proto_id| self.get_proto_item(ProtoId::from(proto_id)))
    }

    /// Iterator over the field_ids section.
    pub fn field_ids(&self) -> impl Iterator<Item = Result<FieldIdItem>> + '_ {
        let field_ids_len = self.inner.field_ids_len();
        (0..field_ids_len).map(move |field_id| self.get_field_item(FieldId::from(field_id)))
    }

    /// Iterator over the method_ids section.
    pub fn method_ids(&self) -> impl Iterator<Item = Result<MethodIdItem>> + '_ {
        let method_ids_len = self.inner.method_ids_len();
        (0..method_ids_len).map(move |method_id| self.get_method_item(MethodId::from(method_id)))
    }

    /// Iterator over the method_handles section.
    pub fn method_handles(&self) -> impl Iterator<Item = Result<MethodHandleItem>> + '_ {
        let method_handles_len = self.inner.method_handles_len().unwrap_or(0);
        (0..method_handles_len).map(move |method_handle_id| {
            self.get_method_handle_item(MethodHandleId::from(method_handle_id))
        })
    }

    /// Iterator over the classes
    pub fn classes(&self) -> impl Iterator<Item = Result<Class>> + '_ {
        self.class_defs()
            .map(move |class_def_item| Class::try_from_dex(&self, &class_def_item?))
    }

    /// Returns the `CodeItem` at the offset.
    pub fn get_code_item(&self, code_off: ulong) -> Result<Option<CodeItem>> {
        if code_off == 0 {
            return Ok(None);
        }
        if !self.is_offset_in_data_section(code_off as uint) {
            return Err(Error::BadOffset(
                code_off as usize,
                "CodeItem offset not in data section".to_string(),
            ));
        }
        Ok(Some(self.source.pread_with(code_off as usize, self)?))
    }

    /// Returns the `AnnotationItem` at the offset.
    pub fn get_annotation_item(&self, annotation_off: uint) -> Result<AnnotationItem> {
        debug!(target: "annotaion-item", "annotation item offset: {}", annotation_off);
        if !self.is_offset_in_data_section(annotation_off) {
            return Err(Error::BadOffset(
                annotation_off as usize,
                "AnnotationItem offset not in data section".to_string(),
            ));
        }
        Ok(self.source.pread_with(annotation_off as usize, self)?)
    }

    /// Returns the `AnnotationSetItem` at the offset.
    pub fn get_annotation_set_item(
        &self,
        annotation_set_item_off: uint,
    ) -> Result<AnnotationSetItem> {
        debug!(target: "annotation-set-item", "annotation set item offset: {}", annotation_set_item_off);
        if annotation_set_item_off == 0 {
            return Ok(Default::default());
        }
        if !self.is_offset_in_data_section(annotation_set_item_off) {
            return Err(Error::BadOffset(
                annotation_set_item_off as usize,
                "AnnotationSetItem offset not in data section".to_string(),
            ));
        }
        self.source
            .pread_with(annotation_set_item_off as usize, self)
    }

    /// Returns the `AnnotationSetRefList` at the offset.
    pub fn get_annotation_set_ref_list(
        &self,
        annotation_set_ref_list_off: uint,
    ) -> Result<AnnotationSetRefList> {
        if !self.is_offset_in_data_section(annotation_set_ref_list_off) {
            return Err(Error::BadOffset(
                annotation_set_ref_list_off as usize,
                "AnnotationSetRefList offset not in data section".to_string(),
            ));
        }
        Ok(self
            .source
            .pread_with(annotation_set_ref_list_off as usize, self)?)
    }

    /// Returns the `EncodedArray` representing the static values of a class at the given offset.
    pub fn get_static_values(&self, static_values_off: uint) -> Result<EncodedArray> {
        debug!(target: "class", "static values offset: {}", static_values_off);
        if static_values_off == 0 {
            return Ok(Default::default());
        }
        if !self.is_offset_in_data_section(static_values_off) {
            return Err(Error::BadOffset(
                static_values_off as usize,
                "Class static values offset not in data section".to_string(),
            ));
        }
        self.source.pread_with(static_values_off as usize, self)
    }

    /// Returns the `AnnotationsDirectoryItem` at the offset.
    pub fn get_annotations_directory_item(
        &self,
        annotations_directory_item_off: uint,
    ) -> Result<AnnotationsDirectoryItem> {
        debug!(target: "class", "annotations directory offset: {}", annotations_directory_item_off);
        if annotations_directory_item_off == 0 {
            return Ok(Default::default());
        }
        if !self.is_offset_in_data_section(annotations_directory_item_off) {
            return Err(Error::BadOffset(
                annotations_directory_item_off as usize,
                "Annotations directory offset not in data section".to_string(),
            ));
        }
        self.source
            .pread_with(annotations_directory_item_off as usize, self)
    }

    /// Returns the `DebugInfoItem` at the offset.
    pub fn get_debug_info_item(&self, debug_info_off: uint) -> Result<DebugInfoItem> {
        if !self.is_offset_in_data_section(debug_info_off) {
            return Err(Error::BadOffset(
                debug_info_off as usize,
                "DebugInfoItem offset not in data section".to_string(),
            ));
        }

        Ok(self.source.pread_with(debug_info_off as usize, self)?)
    }
}

/// Reader facade for loading a `Dex`
pub struct DexReader;

impl DexReader {
    /// Try to read a `Dex` from the given path, returns error if
    /// the file is not a dex or in case of I/O errors
    pub fn from_file<P: AsRef<Path>>(file: P) -> Result<Dex<Mmap>> {
        let map = unsafe { MmapOptions::new().map(&File::open(file.as_ref())?)? };
        let inner: DexInner = map.pread(0)?;
        let endian = inner.endian();
        let source = Source::new(map);
        let cache = Strings::new(
            source.clone(),
            endian,
            inner.strings_offset(),
            inner.strings_len(),
            4096,
            inner.data_section(),
        );
        Ok(Dex {
            source: source.clone(),
            strings: cache,
            inner,
        })
    }

    /// Loads a `Dex` from a `Vec<u8>`
    pub fn from_vec<B: AsRef<[u8]>>(buf: B) -> Result<Dex<B>> {
        let inner: DexInner = buf.as_ref().pread(0)?;
        let endian = inner.endian();
        let source = Source::new(buf);
        let cache = Strings::new(
            source.clone(),
            endian,
            inner.strings_offset(),
            inner.strings_len(),
            4096,
            inner.data_section(),
        );
        Ok(Dex {
            source: source.clone(),
            strings: cache,
            inner,
        })
    }
}

#[cfg(test)]
mod tests {

    use memmap::MmapOptions;
    use std::fs::File;
    use super::Result;
    use std::path::Path;

    #[test]
    fn test_find_class_by_name() {
        let dex =
            super::DexReader::from_file("resources/classes.dex").expect("cannot open dex file");
        let mut count = 0;
        for class_def in dex.class_defs() {
            let class_def = class_def.expect("can't load class");
            let jtype = dex.get_type(class_def.class_idx()).expect("bad type");
            let result = dex.find_class_by_name(&jtype.type_descriptor().to_string());
            assert!(result.is_ok());
            assert!(result.unwrap().is_some());
            count += 1;
        }
        assert!(count > 0);
    }

    fn load_example_dex_as_vec<P: AsRef<Path>>(file: P) -> Result<Vec<u8>> {
        let map = unsafe { MmapOptions::new().map(&File::open(file.as_ref())?)? };
        let data = map.to_vec();
        Ok(data)
    }

    #[test]
    fn test_find_class_by_name_from_vec() {
        let data: Vec<u8> = load_example_dex_as_vec("resources/classes.dex")
            .expect("Cannot load example file to a vec");
        let dex = super::DexReader::from_vec(data).expect("Cannot parse dex from vec");
        let mut count = 0;
        for class_def in dex.class_defs() {
            let class_def = class_def.expect("can't load class");
            let jtype = dex.get_type(class_def.class_idx()).expect("bad type");
            let result = dex.find_class_by_name(&jtype.type_descriptor().to_string());
            assert!(result.is_ok());
            assert!(result.unwrap().is_some());
            count += 1;
        }
        assert!(count > 0);
    }

    #[test]
    fn test_get_type_from_descriptor() {
        let dex =
            super::DexReader::from_file("resources/classes.dex").expect("cannot open dex file");
        let jtype = dex.get_type_from_descriptor("Lorg/adw/launcher/Launcher;");
        assert!(jtype.is_ok());
        let jtype = jtype.unwrap();
        assert!(jtype.is_some());
        let jtype = jtype.unwrap();
        assert_eq!(jtype.type_descriptor(), "Lorg/adw/launcher/Launcher;")
    }
}
