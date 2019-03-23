#[macro_use]
extern crate scroll_derive;

use std::clone::Clone;
use std::fs::File;

use memmap::{Mmap, MmapOptions};
use scroll::{self, ctx, Pread};

use cache::Ref;
use class::{Class, ClassDefItemIter, ClassId};
use jtype::{Type, TypeId};
use source::Source;
use string::{JString, StringCache};

mod cache;
mod class;
mod error;
mod jtype;
mod source;
mod string;

const NO_INDEX: u32 = 0xffffffff;

type Result<T> = std::result::Result<T, error::Error>;

#[derive(Debug, Pread)]
struct Header {
    magic: [u8; 8],
    checksum: u32,
    signature: [u8; 20],
    file_size: u32,
    header_size: u32,
    endian_tag: [u8; 4],
    link_size: u32,
    link_off: u32,
    map_off: u32,
    string_ids_size: u32,
    string_ids_off: u32,
    type_ids_size: u32,
    type_ids_off: u32,
    proto_ids_size: u32,
    proto_ids_off: u32,
    field_ids_size: u32,
    field_ids_off: u32,
    method_ids_size: u32,
    method_ids_off: u32,
    class_defs_size: u32,
    class_defs_off: u32,
    data_size: u32,
    data_off: u32,
}

#[allow(unused)]
pub struct Dex<T> {
    source: Source<T>,
    string_cache: StringCache<T>,
    inner: DexInner,
}

#[derive(Debug)]
struct DexInner {
    header: Header,
    endian: Endian,
}

impl DexInner {
    fn get_endian(&self) -> Endian {
        self.endian
    }

    fn strings_offset(&self) -> u32 {
        self.header.string_ids_off
    }

    fn strings_len(&self) -> u32 {
        self.header.string_ids_size
    }

    fn classes_offset(&self) -> u32 {
        self.header.class_defs_off
    }

    fn class_defs_len(&self) -> u32 {
        self.header.class_defs_size
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
        Ok((DexInner { header, endian }, 0))
    }
}

pub type Endian = scroll::Endian;

impl<T> Dex<T>
where
    T: AsRef<[u8]>,
{
    pub fn from_file(file: &File) -> Result<Dex<Mmap>> {
        let map = unsafe { MmapOptions::new().map(&file)? };
        let inner: DexInner = map.pread(0)?;
        let source = Source::new(map);
        let cache = StringCache::new(
            source.clone(),
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

    fn get_source_file(&self, file_id: string::StringId) -> Result<Option<Ref<JString>>> {
        if file_id == NO_INDEX {
            Ok(None)
        } else {
            Ok(Some(self.get_string(file_id)?))
        }
    }

    fn get_string(&self, id: string::StringId) -> Result<Ref<JString>> {
        self.string_cache.get(id)
    }

    pub fn get_type(&self, type_id: TypeId) -> Result<Type> {
        self.get_string(type_id).map(|string| Type(type_id, string))
    }

    pub fn get_class(&self, _class_id: ClassId) -> Result<Class> {
        unimplemented!()
    }

    pub fn get_class_by_name(&self, jtype: &Type) -> Option<Result<Class>> {
        self.classes_iter()
            .filter(|class| match class {
                Ok(c) => c.get_type() == *jtype,
                Err(_) => false,
            })
            .take(1)
            .next()
    }

    fn get_interfaces(&self, offset: u32) -> Result<Option<Vec<Type>>> {
        let mut offset = offset as usize;
        if offset == 0 {
            return Ok(None);
        }
        let source = &self.source.as_ref().as_ref();
        let endian = self.get_endian();
        let len = source.gread_with::<u32>(&mut offset, endian)?;
        let mut types = Vec::with_capacity(len as usize);
        for _ in 0..len as usize {
            let type_id = source.gread_with::<u16>(&mut offset, endian)? as u32;
            types.push(Type(type_id, self.string_cache.get(type_id)?));
        }
        Ok(Some(types))
    }

    fn get_class_data(&self, mut offset: usize) -> Result<Option<class::ClassDataItem>> {
        if offset == 0 {
            Ok(None)
        } else {
            unimplemented!()
        }
    }

    pub fn get_endian(&self) -> Endian {
        self.inner.get_endian()
    }

    pub fn classes_iter(&self) -> impl Iterator<Item = Result<Class>> + '_ {
        let defs_len = self.inner.class_defs_len();
        let defs_offset = self.inner.classes_offset();
        let source = self.source.clone();
        let endian = self.get_endian();
        ClassDefItemIter::new(source.clone(), defs_offset, defs_len, endian)
            .map(move |class_def_item| Class::from_item(&self, class_def_item?))
    }
}
