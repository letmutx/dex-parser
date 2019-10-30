use scroll::ctx;
use scroll::Pread;
use scroll::Uleb128;

use crate::encoded_value::EncodedValue;
use crate::error::Error;
use crate::field::FieldId;
use crate::jtype::TypeId;
use crate::method::MethodId;
use crate::string::StringId;
use crate::{ubyte, uint};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug)]
pub struct EncodedAnnotation {
    type_idx: TypeId,
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
        let elements = try_gread_vec_with!(source, offset, size, ctx);
        Ok((Self { type_idx, elements }, *offset))
    }
}

#[derive(Debug)]
pub struct AnnotationElement {
    name_idx: StringId,
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
        let value = source.gread_with(offset, ctx)?;
        Ok((Self { name_idx, value }, *offset))
    }
}

#[derive(Debug, FromPrimitive)]
pub enum Visibility {
    Build = 0x0,
    Runtime = 0x1,
    System = 0x2,
}

#[derive(Debug)]
pub struct AnnotationItem {
    visibility: Visibility,
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
        let visibility: Visibility =
            FromPrimitive::from_u8(visibility).expect("Invalid visibility in annotation");
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
struct ParameterAnnotation {
    method_idx: MethodId,
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
        Ok((
            Self {
                method_idx: MethodId::from(method_idx),
                annotations: ctx.get_annotation_set_ref_list(annotation_set_ref_list_off)?,
            },
            *offset,
        ))
    }
}

#[derive(Debug)]
struct MethodAnnotation {
    method_idx: MethodId,
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
        Ok((
            Self {
                method_idx: MethodId::from(method_idx),
                annotations: ctx.get_annotation_set_item(annotation_set_item_off)?,
            },
            *offset,
        ))
    }
}

#[derive(Debug)]
struct FieldAnnotation {
    field_idx: FieldId,
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
        Ok((
            Self {
                field_idx: FieldId::from(field_idx),
                annotations: ctx.get_annotation_set_item(annotation_set_item_off)?,
            },
            *offset,
        ))
    }
}

#[derive(Debug)]
pub(crate) struct AnnotationsDirectoryItem {
    class_annotations: Option<AnnotationSetItem>,
    field_annotations: Option<Vec<FieldAnnotation>>,
    method_annotations: Option<Vec<MethodAnnotation>>,
    parameter_annotations: Option<Vec<ParameterAnnotation>>,
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
        let class_annotations = if class_annotations_off != 0 {
            Some(ctx.get_annotation_set_item(class_annotations_off)?)
        } else {
            None
        };
        let field_annotations = if fields_size != 0 {
            Some(try_gread_vec_with!(source, offset, fields_size, ctx))
        } else {
            None
        };
        let method_annotations = if annotated_method_size != 0 {
            Some(try_gread_vec_with!(
                source,
                offset,
                annotated_method_size,
                ctx
            ))
        } else {
            None
        };
        let parameter_annotations = if annotated_parameters_size != 0 {
            Some(try_gread_vec_with!(
                source,
                offset,
                annotated_parameters_size,
                ctx
            ))
        } else {
            None
        };
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
