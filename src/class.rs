use std::clone::Clone;

use getset::{CopyGetters, Getters};
use scroll::ctx;
use scroll::{Pread, Uleb128};

use crate::annotation::AnnotationsDirectoryItem;
use crate::cache::Ref;
use crate::encoded_item::EncodedItemArrayCtx;
use crate::encoded_value::EncodedArray;
use crate::error::Error;
use crate::field::EncodedFieldArray;
use crate::field::Field;
use crate::jtype::Type;
use crate::method::EncodedMethodArray;
use crate::method::Method;
use crate::source::Source;
use crate::string::JString;
use crate::uint;
use crate::ClassAccessFlags;

pub type ClassId = uint;

#[derive(Debug, Getters, CopyGetters)]
pub struct Class {
    /// Index into type_ids. TypeId should refer to a class type.
    #[get_copy = "pub"]
    pub(crate) id: ClassId,
    /// Type of this class.
    #[get = "pub"]
    pub(crate) jtype: Type,
    /// Access flags for the class (public, final etc.)
    /// https://source.android.com/devices/tech/dalvik/dex-format#access-flags
    #[get_copy = "pub"]
    pub(crate) access_flags: ClassAccessFlags,
    /// Index into the type_ids for the super class, if there is one.
    #[get_copy = "pub"]
    pub(crate) super_class: Option<ClassId>,
    /// List of the interfaces implemented by this class.
    #[get = "pub"]
    pub(crate) interfaces: Option<Vec<Type>>,
    /// Annotations of the class, fields, methods and their parameters.
    #[get = "pub"]
    pub(crate) annotations: Option<AnnotationsDirectoryItem>,
    /// The file in which this class is found in the source code.
    #[get = "pub"]
    pub(crate) source_file: Option<Ref<JString>>,
    /// Static fields defined in the class.
    pub(crate) static_fields: Vec<Field>,
    /// Instance fields defined in the class.
    pub(crate) instance_fields: Vec<Field>,
    /// List of static, private methods and constructors defined in the class.
    pub(crate) direct_methods: Vec<Method>,
    /// List of parent class methods overriden by this class.
    pub(crate) virtual_methods: Vec<Method>,
    /// Values of the static fields in the same order as static fields.
    /// Other static fields assume `0` or `null` values.
    #[get = "pub"]
    pub(crate) static_values: EncodedArray,
}

impl Class {
    /// Static fields defined in the class.
    pub fn static_fields(&self) -> impl Iterator<Item = &Field> + '_ {
        self.static_fields.iter()
    }

    /// Instance fields defined in the class.
    pub fn instance_fields(&self) -> impl Iterator<Item = &Field> + '_ {
        self.instance_fields.iter()
    }

    /// List of static, private methods and constructors defined in the class.
    pub fn direct_methods(&self) -> impl Iterator<Item = &Method> + '_ {
        self.direct_methods.iter()
    }

    /// List of parent class methods overriden by this class.
    pub fn virtual_methods(&self) -> impl Iterator<Item = &Method> + '_ {
        self.virtual_methods.iter()
    }

    /// List of fields defined in this class.
    pub fn fields(&self) -> impl Iterator<Item = &Field> + '_ {
        self.static_fields().chain(self.instance_fields())
    }

    /// List of methods defined in this class.
    pub fn methods(&self) -> impl Iterator<Item = &Method> + '_ {
        self.direct_methods().chain(self.virtual_methods())
    }

    pub(crate) fn try_from_dex<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        class_def: &ClassDefItem,
    ) -> super::Result<Self> {
        debug!(target: "class", "trying to load class: {}", class_def.class_idx);
        let jtype = dex.get_type(class_def.class_idx)?;

        debug!(target: "class", "class: {}, jtype: {}", class_def.class_idx, jtype);

        let data_off = class_def.class_data_off;

        let (static_fields, instance_fields, direct_methods, virtual_methods) = dex
            .get_class_data(data_off)?
            .map(|c| {
                let ef = |encoded_field| dex.get_field(&encoded_field);
                let em = |encoded_method| dex.get_method(&encoded_method);
                Ok((
                    try_from_item!(c.static_fields, ef),
                    try_from_item!(c.instance_fields, ef),
                    try_from_item!(c.direct_methods, em),
                    try_from_item!(c.virtual_methods, em),
                ))
            })
            .unwrap_or_else(|| Ok::<_, Error>((Vec::new(), Vec::new(), Vec::new(), Vec::new())))?;

        let static_values = dex.get_static_values(class_def.static_values_off)?;

        let annotations = dex.get_annotations_directory_item(class_def.annotations_off)?;
        debug!(target: "class", "super class id: {}", class_def.superclass_idx);
        let super_class = if class_def.superclass_idx == super::NO_INDEX {
            Some(class_def.superclass_idx)
        } else {
            None
        };
        debug!(target: "class", "access flags: {}", class_def.access_flags);

        Ok(Class {
            id: class_def.class_idx,
            jtype,
            super_class,
            interfaces: dex.get_interfaces(class_def.interfaces_off)?,
            access_flags: ClassAccessFlags::from_bits(class_def.access_flags).ok_or_else(|| {
                Error::InvalidId(format!(
                    "Invalid Access flags in class {}",
                    class_def.class_idx
                ))
            })?,
            source_file: dex.get_source_file(class_def.source_file_idx)?,
            annotations,
            static_fields,
            instance_fields,
            direct_methods,
            virtual_methods,
            static_values,
        })
    }
}

/// Contains the details about fields and methods of a class.
/// https://source.android.com/devices/tech/dalvik/dex-format#class-data-item
pub(crate) struct ClassDataItem {
    static_fields: Option<EncodedFieldArray>,
    instance_fields: Option<EncodedFieldArray>,
    direct_methods: Option<EncodedMethodArray>,
    virtual_methods: Option<EncodedMethodArray>,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for ClassDataItem
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], dex: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let static_field_size = Uleb128::read(source, offset)?;
        let instance_field_size = Uleb128::read(source, offset)?;
        let direct_methods_size = Uleb128::read(source, offset)?;
        let virtual_methods_size = Uleb128::read(source, offset)?;

        debug!(target: "class data", "static-fields: {}, instance-fields: {}, direct-methods: {}, virtual-methods: {}",
            static_field_size, instance_field_size, direct_methods_size, virtual_methods_size);

        Ok((
            ClassDataItem {
                static_fields: encoded_array!(source, dex, offset, static_field_size),
                instance_fields: encoded_array!(source, dex, offset, instance_field_size),
                direct_methods: encoded_array!(source, dex, offset, direct_methods_size),
                virtual_methods: encoded_array!(source, dex, offset, virtual_methods_size),
            },
            *offset,
        ))
    }
}

/// https://source.android.com/devices/tech/dalvik/dex-format#class-def-item
#[derive(Copy, Clone, Debug, Pread)]
pub(crate) struct ClassDefItem {
    pub(crate) class_idx: uint,
    pub(crate) access_flags: uint,
    pub(crate) superclass_idx: uint,
    pub(crate) interfaces_off: uint,
    pub(crate) source_file_idx: uint,
    pub(crate) annotations_off: uint,
    pub(crate) class_data_off: uint,
    pub(crate) static_values_off: uint,
}

/// Iterator over the class_def_items in the class_defs section.
pub(crate) struct ClassDefItemIter<T> {
    /// Source file of the parent `Dex`.
    source: Source<T>,
    offset: usize,
    len: uint,
    endian: super::Endian,
}

impl<T> ClassDefItemIter<T> {
    pub(crate) fn new(source: Source<T>, offset: uint, len: uint, endian: super::Endian) -> Self {
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
            .gread_with(&mut self.offset, self.endian)
            .map_err(Error::from);
        self.len -= 1;
        Some(class_item)
    }
}
