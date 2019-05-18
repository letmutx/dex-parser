use std::convert::AsRef;
use std::convert::Into;
use std::ops::Deref;

use cesu8::from_java_cesu8;
use scroll::{self, ctx, Pread, Uleb128};

use crate::cache::{Cache, Ref};
use crate::error;
use crate::error::Error;
use crate::source::Source;
use crate::Result;

pub type StringId = u32;

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
    type Target = str;

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

pub(crate) struct StringCache<T> {
    source: Source<T>,
    offset: u32,
    len: u32,
    inner: Cache<StringId, JString>,
}

impl<T> StringCache<T>
where
    T: AsRef<[u8]>,
{
    pub(crate) fn new(source: Source<T>, offset: u32, len: u32, cache_size: usize) -> Self {
        Self {
            source,
            offset,
            len,
            inner: Cache::new(cache_size),
        }
    }

    fn parse(&self, id: StringId) -> Result<JString> {
        let source = self.source.as_ref().as_ref();
        let string_data_off: u32 = source.pread((self.offset + id) as usize)?;
        self.source
            .as_ref()
            .as_ref()
            .pread(string_data_off as usize)
    }

    pub(crate) fn get(&self, id: StringId) -> Result<Ref<JString>> {
        if id > self.len {
            return Err(Error::InvalidId("Invalid string id".to_string()));
        }
        if let Some(string) = self.inner.get(&id) {
            Ok(string)
        } else {
            self.inner.put(id, self.parse(id)?);
            Ok(self.inner.get(&id).unwrap())
        }
    }
}

impl<T> Clone for StringCache<T> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            offset: self.offset,
            len: self.len,
            inner: self.inner.clone(),
        }
    }
}
