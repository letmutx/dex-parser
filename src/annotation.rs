//! Structures for Annotations on a `Class`, `Method`, `MethodParams` and `Field`s.
use scroll::ctx;
use scroll::Pread;
use scroll::Uleb128;

use getset::{CopyGetters, Getters};

use crate::encoded_value::EncodedValue;
use crate::error::Error;
use crate::field::FieldId;
use crate::jtype::TypeId;
use crate::method::MethodId;
use crate::string::StringId;
use crate::{ubyte, uint};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// Contains the type and parameters of an Annotation.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#encoded-annotation)
#[derive(Debug, Getters, CopyGetters, PartialEq)]
pub struct EncodedAnnotation {
    /// Type of the annotation. Should be a class type.
    #[get_copy = "pub"]
    type_idx: TypeId,
    /// Elements of the annotation
    #[get = "pub"]
    elements: Vec<AnnotationElement>,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for EncodedAnnotation
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let type_idx = Uleb128::read(source, offset)?;
        let type_idx = type_idx as TypeId;
        let size = Uleb128::read(source, offset)?;
        debug!(target: "encoded-annotation", "type: {}, size: {}", type_idx, size);
        let elements = try_gread_vec_with!(source, offset, size, ctx);
        Ok((Self { type_idx, elements }, *offset))
    }
}

/// Represents a parameter of an annotation. For example, if `@Author(name = "Benjamin Franklin")`, is
/// the annotation, this structure represents `name = "Benjamin Franklin"`.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#annotation-element)
#[derive(Debug, Getters, CopyGetters, PartialEq)]
pub struct AnnotationElement {
    /// Name of the element. Should conform to
    /// https://source.android.com/devices/tech/dalvik/dex-format#membername
    #[get_copy = "pub"]
    name_idx: StringId,
    /// Value corresponding to the name.
    #[get = "pub"]
    value: EncodedValue,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for AnnotationElement
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let name_idx = Uleb128::read(source, offset)?;
        let name_idx = name_idx as StringId;
        debug!(target: "annotation-element", "annotation element: {}", name_idx);
        let value = source.gread_with(offset, ctx)?;
        Ok((Self { name_idx, value }, *offset))
    }
}

/// Visibility of an annotation.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#visibility)
#[derive(Debug, FromPrimitive, Copy, Clone)]
pub enum Visibility {
    /// Visible only to the Build system.
    Build = 0x0,
    /// Visible at the Runtime.
    Runtime = 0x1,
    /// Visible only to the virtual machine.
    System = 0x2,
}

/// An Annotation along with its visibility.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#annotation-item)
#[derive(Debug, Getters, CopyGetters)]
pub struct AnnotationItem {
    /// Visibility of this annotation.
    #[get_copy = "pub"]
    visibility: Visibility,
    /// Type and parameters of this annotation.
    #[get = "pub"]
    annotation: EncodedAnnotation,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for AnnotationItem
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let visibility: ubyte = source.gread_with(offset, ctx.get_endian())?;
        debug!(target: "annotation-item", "visibility: {:?}", visibility);
        let visibility: Visibility = FromPrimitive::from_u8(visibility)
            .ok_or_else(|| Error::InvalidId("Invalid visibility for annotation".to_owned()))?;
        let annotation = source.gread_with(offset, ctx)?;
        Ok((
            Self {
                visibility,
                annotation,
            },
            *offset,
        ))
    }
}

/// List of Annotation Sets. Used for method parameter annotations.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#set-ref-list)
#[derive(Debug, Default, Getters)]
#[get = "pub"]
pub struct AnnotationSetRefList {
    annotation_set_list: Vec<AnnotationSetItem>,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for AnnotationSetRefList
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let endian = ctx.get_endian();
        let size: uint = source.gread_with(offset, endian)?;
        debug!(target: "annotation-set-ref-list", "annotation set ref list size: {}", size);
        let annotation_ref_items: Vec<uint> = try_gread_vec_with!(source, offset, size, endian);
        Ok((
            Self {
                annotation_set_list: annotation_ref_items
                    .iter()
                    .map(|annotation_set_item_off| {
                        ctx.get_annotation_set_item(*annotation_set_item_off)
                    })
                    .collect::<super::Result<_>>()?,
            },
            *offset,
        ))
    }
}

/// A set of annotations on an element.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#annotation-set-item)
#[derive(Debug, Default, Getters)]
#[get = "pub"]
pub struct AnnotationSetItem {
    annotations: Vec<AnnotationItem>,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for AnnotationSetItem
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let endian = ctx.get_endian();
        let size: uint = source.gread_with(offset, endian)?;
        debug!(target: "annotation-set-item", "annotation set items size: {}", size);
        let annotation_items_offs: Vec<uint> = try_gread_vec_with!(source, offset, size, endian);
        Ok((
            Self {
                annotations: annotation_items_offs
                    .iter()
                    .map(|annotation_off| ctx.get_annotation_item(*annotation_off))
                    .collect::<super::Result<_>>()?,
            },
            *offset,
        ))
    }
}

/// Annotations of a `Method`'s parameters.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#parameter-annotation)
#[derive(Debug, Getters, CopyGetters)]
pub struct ParameterAnnotation {
    /// The method this parameter belongs to.
    #[get_copy = "pub"]
    method_idx: MethodId,
    /// The list of annotation sets for the parameters.
    #[get = "pub"]
    annotations: AnnotationSetRefList,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for ParameterAnnotation
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let endian = ctx.get_endian();
        let method_idx: uint = source.gread_with(offset, endian)?;
        let annotation_set_ref_list_off: uint = source.gread_with(offset, endian)?;
        debug!(target: "parameter-annotation", "annotation set ref list offset: {}", annotation_set_ref_list_off);
        Ok((
            Self {
                method_idx: MethodId::from(method_idx),
                annotations: ctx.get_annotation_set_ref_list(annotation_set_ref_list_off)?,
            },
            *offset,
        ))
    }
}

/// Annotations of a `Method`.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#method-annotation)
#[derive(Debug, Getters, CopyGetters)]
pub struct MethodAnnotation {
    #[get_copy = "pub"]
    method_idx: MethodId,
    #[get = "pub"]
    annotations: AnnotationSetItem,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for MethodAnnotation
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let method_idx: uint = source.gread_with(offset, ctx.get_endian())?;
        let annotation_set_item_off: uint = source.gread_with(offset, ctx.get_endian())?;
        debug!(target: "method-annotation", "annotation set item offset: {}", annotation_set_item_off);
        Ok((
            Self {
                method_idx: MethodId::from(method_idx),
                annotations: ctx.get_annotation_set_item(annotation_set_item_off)?,
            },
            *offset,
        ))
    }
}

/// Annotations of a `Field`.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#field-annotation)
#[derive(Debug, Getters, CopyGetters)]
pub struct FieldAnnotation {
    #[get_copy = "pub"]
    field_idx: FieldId,
    #[get = "pub"]
    annotations: AnnotationSetItem,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for FieldAnnotation
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let field_idx: uint = source.gread_with(offset, ctx.get_endian())?;
        let annotation_set_item_off: uint = source.gread_with(offset, ctx.get_endian())?;
        debug!(target: "field-annotation", "annotation set item offset: {}", annotation_set_item_off);
        Ok((
            Self {
                field_idx: FieldId::from(field_idx),
                annotations: ctx.get_annotation_set_item(annotation_set_item_off)?,
            },
            *offset,
        ))
    }
}

/// Annotations of the fields, methods and parameters of a class and the class itself.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#annotations-directory)
#[derive(Debug, Default, Getters)]
pub struct AnnotationsDirectoryItem {
    pub(crate) class_annotations: AnnotationSetItem,
    pub(crate) field_annotations: Vec<FieldAnnotations>,
    pub(crate) method_annotations: Vec<MethodAnnotations>,
    pub(crate) parameter_annotations: Vec<ParameterAnnotations>,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for AnnotationsDirectoryItem
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], ctx: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let endian = ctx.get_endian();
        let class_annotations_off: uint = source.gread_with(offset, endian)?;
        let fields_size: uint = source.gread_with(offset, endian)?;
        let annotated_method_size: uint = source.gread_with(offset, endian)?;
        let annotated_parameters_size: uint = source.gread_with(offset, endian)?;
        debug!(target: "annotations directory", "fields size: {}, annotated method size: {}, annotated params size: {}",
            fields_size, annotated_method_size, annotated_parameters_size);
        let class_annotations = ctx.get_annotation_set_item(class_annotations_off)?;
        let field_annotations = try_gread_vec_with!(source, offset, fields_size, ctx);
        let method_annotations = try_gread_vec_with!(source, offset, annotated_method_size, ctx);
        let parameter_annotations =
            try_gread_vec_with!(source, offset, annotated_parameters_size, ctx);
        Ok((
            Self {
                class_annotations,
                field_annotations,
                method_annotations,
                parameter_annotations,
            },
            *offset,
        ))
    }
}
