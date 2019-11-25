//! Dex String utilities
use std::{
    convert::AsRef,
    fmt,
    ops::{Deref, Range},
};

use cesu8::{from_java_cesu8, to_java_cesu8};
use scroll::{self, ctx, Pread, Uleb128};

use crate::{cache::Cache, error, error::Error, source::Source, uint, Result};
use std::rc::Rc;

/// Index into the `StringId`s section.
pub type StringId = uint;

/// Strings in `Dex` file are encoded as MUTF-8 code units. DexString is a
/// wrapper type for converting Dex strings into Rust strings.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#mutf-8)
#[derive(Debug, Hash, Eq, PartialEq, Clone, PartialOrd, Ord)]
pub struct DexString {
    string: Rc<String>,
}

impl PartialEq<str> for DexString {
    fn eq(&self, other: &str) -> bool {
        *self.string == other
    }
}

impl<'a> PartialEq<&'a str> for DexString {
    fn eq(&self, other: &&'a str) -> bool {
        *self.string == *other
    }
}

impl fmt::Display for DexString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.string)
    }
}

impl From<String> for DexString {
    fn from(string: String) -> Self {
        DexString {
            string: Rc::new(string),
        }
    }
}

impl Deref for DexString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.string
    }
}

impl<'a> ctx::TryFromCtx<'a, scroll::Endian> for DexString {
    type Error = error::Error;
    type Size = usize;

    // https://source.android.com/devices/tech/dalvik/dex-format#string-data-item
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
            DexString {
                string: Rc::new(
                    from_java_cesu8(bytes)
                        .map_err(|e| Error::MalFormed(format!("Malformed string: {:?}", e)))?
                        .into_owned(),
                ),
            },
            size,
        ))
    }
}

/// To prevent encoding/decoding Java strings to Rust strings
/// every time, we cache the strings in memory. This also potentially
/// reduces I/O because strings are used in a lot of places.
pub(crate) struct Strings<T> {
    source: Source<T>,
    ///  Offset into the strings section.
    offset: uint,
    endian: super::Endian,
    /// Length of the strings section.
    len: uint,
    cache: Cache<StringId, DexString>,
    data_section: Range<uint>,
}

impl<T> Strings<T>
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
        data_section: Range<uint>,
    ) -> Self {
        Self {
            source,
            offset,
            endian,
            len,
            cache: Cache::new(cache_size),
            data_section,
        }
    }

    fn parse(&self, id: StringId) -> Result<DexString> {
        let source = &self.source;
        let offset = self.offset as usize + id as usize * 4;
        let string_data_off: uint = source.pread_with(offset, self.endian)?;
        if !self.data_section.contains(&string_data_off) {
            return Err(error::Error::BadOffset(
                string_data_off as usize,
                format!("string_data_off not in data section for StringId: {}", id),
            ));
        }
        source.pread(string_data_off as usize)
    }

    /// Get the string at `id` updating the cache with the new item
    pub(crate) fn get(&self, id: StringId) -> Result<DexString> {
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

    pub(crate) fn get_id(&self, string: &str) -> Result<Option<StringId>> {
        use crate::search::Section;
        let java_string = to_java_cesu8(string);
        let (offset, len) = (self.offset as usize, self.len as usize);
        let string_section = &self.source[offset..offset + len * std::mem::size_of::<StringId>()];
        let section = Section::new(string_section);
        let source = self.source.clone();
        let index = section.binary_search(
            &java_string,
            self.endian,
            move |data_offset: &uint, element: &std::borrow::Cow<[u8]>| {
                let mut data_offset = *data_offset as usize;
                let _ = Uleb128::read(source.as_ref(), &mut data_offset)
                    .map_err(crate::error::Error::from)?;
                let value = &source[data_offset..data_offset + element.len()];
                Ok((**element).cmp(value))
            },
        )?;
        Ok(index.map(|i| i as StringId))
    }
}

impl<T> Clone for Strings<T> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            offset: self.offset,
            endian: self.endian,
            len: self.len,
            cache: self.cache.clone(),
            data_section: self.data_section.clone(),
        }
    }
}

/// Iterator over the strings in the strings section.
pub struct StringsIter<T> {
    /// String cache shared by the parent `Dex`
    cache: Strings<T>,
    current: usize,
    len: usize,
}

impl<T: AsRef<[u8]>> StringsIter<T> {
    pub(crate) fn new(cache: Strings<T>, len: usize) -> Self {
        Self {
            cache,
            current: 0,
            len,
        }
    }
}

impl<T: AsRef<[u8]>> Iterator for StringsIter<T> {
    type Item = super::Result<DexString>;

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

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_string() {
        let dex = crate::DexReader::from_file("resources/classes.dex").expect("failed to open dex");
        let value = dex.strings.get_id("Lorg/adw/launcher/Launcher;");
        assert!(value.is_ok());
        let value = value.unwrap();
        assert!(value.is_some());
        let string_id = value.unwrap();
        assert_eq!(
            dex.get_string(string_id)
                .expect("string id doesn't exist")
                .to_string(),
            "Lorg/adw/launcher/Launcher;"
        );
    }
}
