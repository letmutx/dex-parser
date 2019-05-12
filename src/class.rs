use std::clone::Clone;

use scroll::{Pread, Uleb128};

use crate::cache::Ref;
use crate::encoded_item::EncodedItem;
use crate::encoded_item::{EncodedItemArray, EncodedItemArrayCtx};
use crate::error::Error;
use crate::field::EncodedFieldArray;
use crate::field::Field;
use crate::jtype::Type;
use crate::method::EncodedMethodArray;
use crate::method::Method;
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
    pub(crate) instance_fields: Option<Vec<Field>>,
    pub(crate) direct_methods: Option<Vec<Method>>,
    pub(crate) virtual_methods: Option<Vec<Method>>,
}

fn into_item<T, F, U>(array: Option<EncodedItemArray<T>>, f: F) -> Option<super::Result<Vec<U>>>
where
    F: Fn(T) -> super::Result<U>,
    T: EncodedItem,
{
    array.map(|array| array.into_iter().map(f).collect())
}

impl Class {
    pub(crate) fn from_dex<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        class_def: &ClassDefItem,
    ) -> super::Result<Self> {
        let data_off = class_def.class_data_off;

        let (static_fields, instance_fields, direct_methods, virtual_methods) =
            match dex.get_class_data(data_off)? {
                Some(c) => {
                    let ClassDataItem {
                        static_fields,
                        instance_fields,
                        direct_methods,
                        virtual_methods,
                    } = c;

                    let ec = |encoded_field| dex.get_field(&encoded_field);
                    let ef = |encoded_method| dex.get_method(&encoded_method);

                    let static_fields = match into_item(static_fields, ec) {
                        Some(v) => Some(v?),
                        None => None,
                    };
                    let instance_fields = match into_item(instance_fields, ec) {
                        Some(v) => Some(v?),
                        None => None,
                    };
                    let direct_methods = match into_item(direct_methods, ef) {
                        Some(v) => Some(v?),
                        None => None,
                    };
                    let virtual_methods = match into_item(virtual_methods, ef) {
                        Some(v) => Some(v?),
                        None => None,
                    };
                    (
                        static_fields,
                        instance_fields,
                        direct_methods,
                        virtual_methods,
                    )
                }
                None => (None, None, None, None),
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
            direct_methods,
            virtual_methods,
        })
    }

    pub fn get_type(&self) -> Type {
        self.jtype.clone()
    }
}

pub(crate) struct ClassDataItem {
    static_fields: Option<EncodedFieldArray>,
    instance_fields: Option<EncodedFieldArray>,
    direct_methods: Option<EncodedMethodArray>,
    virtual_methods: Option<EncodedMethodArray>,
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
            Some(source.gread_with::<EncodedFieldArray>(offset, encoded_array_ctx)?)
        } else {
            None
        };

        let instance_fields = if instance_field_size > 0 {
            let encoded_array_ctx = EncodedItemArrayCtx::new(dex, instance_field_size as usize);
            Some(source.gread_with::<EncodedFieldArray>(offset, encoded_array_ctx)?)
        } else {
            None
        };

        let direct_methods = if direct_methods_size > 0 {
            let encoded_array_ctx = EncodedItemArrayCtx::new(dex, direct_methods_size as usize);
            Some(source.gread_with::<EncodedMethodArray>(offset, encoded_array_ctx)?)
        } else {
            None
        };

        let virtual_methods = if virtual_methods_size > 0 {
            let encoded_array_ctx = EncodedItemArrayCtx::new(dex, virtual_methods_size as usize);
            Some(source.gread_with::<EncodedMethodArray>(offset, encoded_array_ctx)?)
        } else {
            None
        };

        Ok(Some(ClassDataItem {
            static_fields,
            instance_fields,
            direct_methods,
            virtual_methods,
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
