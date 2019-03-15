#[macro_use]
extern crate scroll_derive;

use memmap::{Mmap, MmapOptions};
use scroll::{self, ctx, Pread};
use std::clone::Clone;
use std::fs::File;

use source::Source;
use string::StringCache;

mod cache;
mod error;
mod source;
mod string;

type Result<T> = std::result::Result<T, error::Error>;

#[derive(Debug, Pread)]
pub struct Header {
    pub magic: [u8; 8],
    checksum: u32,
    signature: [u8; 20],
    pub file_size: u32,
    pub header_size: u32,
    pub endian_tag: [u8; 4],
    pub link_size: u32,
    pub link_off: u32,
    pub map_off: u32,
    pub string_ids_size: u32,
    pub string_ids_off: u32,
    pub type_ids_size: u32,
    pub type_ids_off: u32,
    pub proto_ids_size: u32,
    pub proto_ids_off: u32,
    pub field_ids_size: u32,
    pub field_ids_off: u32,
    pub method_ids_size: u32,
    pub method_ids_off: u32,
    pub class_defs_size: u32,
    pub class_defs_off: u32,
    pub data_size: u32,
    pub data_off: u32,
}

pub type TypeId = usize;

pub struct Dex<T> {
    source: Source<T>,
    string_cache: StringCache<T>,
    inner: DexInner,
}

#[derive(Debug)]
struct DexInner {
    header: Header,
}

impl DexInner {
    fn str_table_offset(&self) -> usize {
        self.header.string_ids_off as usize
    }

    fn str_table_len(&self) -> usize {
        self.header.string_ids_size as usize
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
        Ok((DexInner { header }, 0))
    }
}

pub struct Type;

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
            inner.str_table_offset(),
            inner.str_table_len(),
            4096,
        );
        Ok(Dex {
            source: source.clone(),
            string_cache: cache,
            inner: inner,
        })
    }

    pub fn get_type(&self, _type_id: TypeId) -> Result<Type> {
        unimplemented!()
    }
}
