use crate::error;
use crate::Result;
use cesu8::from_java_cesu8;

use scroll::{self, ctx, Pread, Uleb128};
use std::convert::AsRef;
use std::convert::Into;
use std::ops::Deref;
use std::rc::Rc;

use crate::cache::Cache;
use crate::error::Error;
use crate::source::Source;

pub type StringId = usize;

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
        JString { string: string }
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
    offset: usize,
    len: usize,
    inner: Cache<StringId, JString>,
}

impl<T> StringCache<T>
where
    T: AsRef<[u8]>,
{
    pub(crate) fn new(source: Source<T>, offset: usize, len: usize, cache_size: usize) -> Self {
        Self {
            source,
            offset,
            len,
            inner: Cache::new(cache_size),
        }
    }

    fn parse(&self, id: StringId) -> Result<JString> {
        self.source
            .as_ref()
            .as_ref()
            .pread(self.offset + id)
            .map_err(error::Error::from)
    }

    pub(crate) fn get(&self, id: StringId) -> Result<Rc<JString>> {
        if id > self.len {
            return Err(Error::InvalidId("Invalid string id".to_string()));
        }
        if let Some(string) = self.inner.get(&id) {
            Ok(string)
        } else {
            match self.parse(id) {
                Ok(string) => {
                    self.inner.put(id, string);
                    Ok(self.inner.get(&id).unwrap())
                }
                Err(e) => Err(e),
            }
        }
    }
}
