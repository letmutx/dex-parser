#[macro_use]
extern crate scroll_derive;

use cesu8::from_java_cesu8;

use scroll::{self, ctx, Pread, Uleb128};
use std::convert::Into;
use std::iter::FromIterator;
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
pub struct JString {
    string: String,
}

impl Into<String> for JString {
    fn into(self) -> String {
        self.string
    }
}

impl From<String> for JString {
    fn from(string: String) -> Self {
        JString { string }
    }
}

impl Deref for JString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.string
    }
}

impl<'a> ctx::TryFromCtx<'a, scroll::Endian> for JString {
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
                string: from_java_cesu8(bytes).unwrap().into_owned(),
            },
            size,
        ))
    }
}

pub struct Type(u32);

#[derive(Debug)]
pub struct Dex {
    header: Header,
    strings: List<JString>,
}

impl<'a> ctx::TryFromCtx<'a, ()> for Dex {
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
        let strings = {
            let ctx = ListCtx {
                size: header.string_ids_size as usize,
                endian,
            };
            let strings: List<u32> = source.pread_with(header.string_ids_off as usize, ctx)?;
            let mut result = List::new();
            for offset in strings.iter() {
                result.push(source.pread::<JString>(*offset as usize)?);
            }
            result
        };

        let types: List<Type> = {
            let ctx = ListCtx {
                size: header.type_ids_size as usize,
                endian,
            };
            let type_ids: List<u32> = source.pread_with(header.type_ids_off as usize, ctx)?;
            type_ids.iter().map(|string_id| Type(*string_id)).collect()
        };

        unimplemented!()
    }
}

#[derive(Clone, Copy, Default)]
struct ListCtx {
    size: usize,
    endian: scroll::Endian,
}

#[derive(Debug)]
struct List<T> {
    inner: Vec<T>,
}

impl<T> FromIterator<T> for List<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut result = List::new();
        iter.into_iter().for_each(|s| result.push(s));
        result
    }
}

impl<T> List<T> {
    fn new() -> Self {
        List { inner: Vec::new() }
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.iter()
    }

    fn push(&mut self, val: T) {
        self.inner.push(val);
    }
}

impl<'a, T> ctx::TryFromCtx<'a, ListCtx> for List<T>
where
    T: ctx::TryFromCtx<'a, scroll::Endian, Size = usize, Error = scroll::Error>,
{
    type Error = error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], context: ListCtx) -> Result<(Self, Self::Size)> {
        let mut inner = Vec::with_capacity(context.size);
        let offset = &mut 0;
        for _ in 0..context.size {
            inner.push(
                source
                    .gread_with::<T>(offset, context.endian)
                    .map_err(error::Error::from)?,
            );
        }
        Ok((List { inner }, *offset))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
