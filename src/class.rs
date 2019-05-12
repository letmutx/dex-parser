use std::clone::Clone;

use scroll::{Pread, Uleb128};

use crate::cache::Ref;
use crate::error::Error;
use crate::field::{EncodedItemArray, EncodedItemArrayCtx};
use crate::field::Field;
use crate::jtype::Type;
use crate::source::Source;
use crate::string::JString;
use crate::field::EncodedField;

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
    pub(crate) instance_fields: Option<Vec<Field>>,
    //    pub(crate) direct_methods: Vec<Method>,
    //    pub(crate) virtual_methods: Vec<Method>
}

impl Class {
    pub(crate) fn from_dex<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        class_def: &ClassDefItem,
    ) -> super::Result<Self> {
        let data_off = class_def.class_data_off;
        let into_field = |ef_array: Option<EncodedItemArray<EncodedField>>| {
            ef_array
                .map(|ef_array| {
                    let result: super::Result<Vec<Field>> = ef_array
                        .into_iter()
                        .map(|encoded_field| dex.get_field(&encoded_field))
                        .collect();
                    result
                })
        };

        let (static_fields, instance_fields) = match dex.get_class_data(data_off)? {
            Some(c) => {
                let ClassDataItem {
                    static_fields,
                    instance_fields,
                } = c;
                
                let static_fields = match into_field(static_fields) {
                    Some(e) => Some(e?),
                    None => None,
                };
                let instance_fields = match into_field(instance_fields) {
                    Some(e) => Some(e?),
                    None => None
                };
                (static_fields, instance_fields)
            }
            None => (None, None),
        };
        Ok(Class {
            id: class_def.class_idx,
            jtype: dex.get_type(class_def.class_idx)?,
            super_class: class_def.superclass_idx,
            interfaces: dex.get_interfaces(class_def.interfaces_off)?,
            access_flags: class_def.access_flags,
            source_file: dex.get_source_file(class_def.source_file_idx)?,
            static_fields,
            instance_fields,
        })
    }

    pub fn get_type(&self) -> Type {
        self.jtype.clone()
    }
}

pub(crate) struct ClassDataItem {
    static_fields: Option<EncodedItemArray<EncodedField>>,
    instance_fields: Option<EncodedItemArray<EncodedField>>,
}

impl ClassDataItem {
    pub(crate) fn from_dex<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        offset: u32,
    ) -> super::Result<Option<Self>> {
        if offset == 0 {
            return Ok(None);
        }
        let offset = &mut (offset as usize);
        let source = &dex.source.as_ref().as_ref();
        let static_field_size = Uleb128::read(source, offset)?;
        let instance_field_size = Uleb128::read(source, offset)?;
        let direct_methods_size = Uleb128::read(source, offset)?;
        let virtual_methods_size = Uleb128::read(source, offset)?;
        // TODO: may be use a macro here
        let static_fields = if static_field_size > 0 {
            let encoded_array_ctx = EncodedItemArrayCtx::new(dex, static_field_size as usize);
            Some(source.gread_with::<EncodedItemArray<EncodedField>>(offset, encoded_array_ctx)?)
        } else {
            None
        };

        let instance_fields = if instance_field_size > 0 {
            let encoded_array_ctx = EncodedItemArrayCtx::new(dex, instance_field_size as usize);
            Some(source.gread_with::<EncodedItemArray<EncodedField>>(offset, encoded_array_ctx)?)
        } else {
            None
        };

        Ok(Some(ClassDataItem {
            static_fields,
            instance_fields,
        }))
    }
}

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
