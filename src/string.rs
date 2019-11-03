use std::convert::AsRef;
use std::convert::Into;
use std::ops::Deref;

use cesu8::from_java_cesu8;
use scroll::{self, ctx, Pread, Uleb128};

use crate::cache::{Cache, Ref};
use crate::error;
use crate::error::Error;
use crate::source::Source;
use crate::uint;
use crate::Result;

pub type StringId = uint;

/// Strings in `Dex` file are encoded as MUTF-8 code units. JString is a
/// wrapper type for converting Java strings into Rust strings.
/// https://source.android.com/devices/tech/dalvik/dex-format#mutf-8
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

    /// https://source.android.com/devices/tech/dalvik/dex-format#string-data-item
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

/// To prevent encoding/decoding Java strings to Rust strings
/// every time, we cache the strings in memory. This also potentially
/// reduces I/O because strings are used in a lot of places.
pub(crate) struct StringCache<T> {
    source: Source<T>,
    ///  Offset into the strings section.
    offset: uint,
    endian: super::Endian,
    /// Length of the strings section.
    len: uint,
    cache: Cache<StringId, JString>,
}

impl<T> StringCache<T>
where
    T: AsRef<[u8]>,
{
    /// Returns a new instance of the string cache
    pub(crate) fn new(
        source: Source<T>,
        endian: super::Endian,
        offset: uint,
        len: uint,
        cache_size: usize,
    ) -> Self {
        Self {
            source,
            offset,
            endian,
            len,
            cache: Cache::new(cache_size),
        }
    }

    fn parse(&self, id: StringId) -> Result<JString> {
        let source = self.source.as_ref();
        let offset = self.offset as usize + id as usize * 4;
        let string_data_off: uint = source.pread_with(offset, self.endian)?;
        source.pread(string_data_off as usize)
    }

    /// Get the string at `id` updating the cache with the new item
    pub(crate) fn get(&self, id: StringId) -> Result<Ref<JString>> {
        if id >= self.len {
            return Err(Error::InvalidId(format!("Invalid string id: {}", id)));
        }
        if let Some(string) = self.cache.get(&id) {
            Ok(string)
        } else {
            self.cache.put(id, self.parse(id)?);
            Ok(self.cache.get(&id).unwrap())
        }
    }
}

impl<T> Clone for StringCache<T> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            offset: self.offset,
            endian: self.endian,
            len: self.len,
            cache: self.cache.clone(),
        }
    }
}

/// Iterator over the strings in the strings section.
pub struct Strings<T> {
    /// String cache shared by the parent `Dex`
    cache: StringCache<T>,
    current: usize,
    len: usize,
}

impl<T: AsRef<[u8]>> Strings<T> {
    pub(crate) fn new(cache: StringCache<T>, len: usize) -> Self {
        Self {
            cache,
            current: 0,
            len,
        }
    }
}

impl<T: AsRef<[u8]>> Iterator for Strings<T> {
    type Item = super::Result<Ref<JString>>;

    // NOTE: iteration may cause cache thrashing, introduce a new
    // method to get but not update cache if needed
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.len {
            return None;
        }
        let next = self.cache.get(self.current as uint);
        self.current += 1;
        Some(next)
    }
}
