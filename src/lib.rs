#[macro_use]
extern crate scroll_derive;

use cesu8::from_java_cesu8;

use memmap::MmapOptions;
use scroll::{self, ctx, Pread, Uleb128};
use std::borrow::Cow;
use std::convert::Into;
use std::fs::File;
use std::ops::Deref;

mod error;

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

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct JString<'a> {
    string: Cow<'a, str>,
}

impl<'a> Into<String> for JString<'a> {
    fn into(self) -> String {
        self.string.into_owned()
    }
}

impl<'a> From<String> for JString<'a> {
    fn from(string: String) -> Self {
        JString {
            string: Cow::from(string),
        }
    }
}

impl<'a> Deref for JString<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.string
    }
}

impl<'a> ctx::TryFromCtx<'a, scroll::Endian> for JString<'a> {
    type Error = error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], _: scroll::Endian) -> Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let _ = Uleb128::read(source, offset)?;
        let count = source
            .iter()
            .skip(*offset)
            .take_while(|c| **c != b'\0')
            .count();
        let bytes = &source[*offset..*offset + count];
        let size = *offset + bytes.len();
        Ok((
            JString {
                string: Cow::from(from_java_cesu8(bytes).unwrap()),
            },
            size,
        ))
    }
}

pub struct Type(u32);

#[derive(Debug)]
pub struct Dex {
    source: memmap::Mmap,
    inner: DexInner,
}

#[derive(Debug)]
struct DexInner {
    header: Header,
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

impl Dex {
    pub fn from_file(file: &File) -> Result<Dex> {
        let map = unsafe { MmapOptions::new().map(&file)? };
        let inner = map.pread(0)?;
        Ok(Dex { source: map, inner })
    }
}
