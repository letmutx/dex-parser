use std::clone::Clone;

use scroll::Pread;

use crate::cache::Ref;
use crate::error::Error;
use crate::jtype::Type;
use crate::source::Source;
use crate::string::JString;

pub type ClassId = u32;
// TODO: define an enum for this
pub type AccessFlags = u32;

#[allow(unused)]
pub struct Class {
    pub(crate) id: ClassId,
    pub(crate) access_flags: AccessFlags,
    pub(crate) super_class: ClassId,
    pub(crate) interfaces: Option<Vec<Type>>,
    pub(crate) jtype: Type,
    pub(crate) source_file: Option<Ref<JString>>,
    pub(crate) static_fields: Option<Vec<Field>>,
    //    pub(crate) instance_fields: Vec<Field>,
    //    pub(crate) direct_methods: Vec<Method>,
    //    pub(crate) virtual_methods: Vec<Method>
}

impl Class {
    pub(crate) fn from_item<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        class_def: ClassDefItem,
    ) -> super::Result<Self> {
        let data_off = class_def.class_data_off;
        let class_data = dex.get_class_data(data_off)?;
        Ok(Class {
            id: class_def.class_idx,
            jtype: dex.get_type(class_def.class_idx)?,
            super_class: class_def.superclass_idx,
            interfaces: dex.get_interfaces(class_def.interfaces_off)?,
            access_flags: class_def.access_flags,
            source_file: dex.get_source_file(class_def.source_file_idx)?,
            static_fields: class_data.static_fields,
        })
    }

    pub fn get_type(&self) -> Type {
        return self.jtype.clone();
    }
}

#[derive(Copy, Clone, Debug, Pread)]
pub(crate) struct ClassDataItem {}

#[derive(Copy, Clone, Debug, Pread)]
pub(crate) struct ClassDefItem {
    pub(crate) class_idx: u32,
    pub(crate) access_flags: u32,
    pub(crate) superclass_idx: u32,
    pub(crate) interfaces_off: u32,
    pub(crate) source_file_idx: u32,
    pub(crate) annotations_off: u32,
    pub(crate) class_data_off: u32,
    pub(crate) static_values_off: u32,
}

pub(crate) struct ClassDefItemIter<T> {
    source: Source<T>,
    offset: usize,
    len: u32,
    endian: super::Endian,
}

impl<T> ClassDefItemIter<T> {
    pub(crate) fn new(source: Source<T>, offset: u32, len: u32, endian: super::Endian) -> Self {
        Self {
            source,
            offset: offset as usize,
            len,
            endian,
        }
    }
}

impl<T: AsRef<[u8]>> Iterator for ClassDefItemIter<T> {
    type Item = super::Result<ClassDefItem>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        let class_item: super::Result<ClassDefItem> = self
            .source
            .as_ref()
            .as_ref()
            .gread_with(&mut self.offset, self.endian)
            .map_err(Error::from);
        self.len -= 1;
        Some(class_item)
    }
}
