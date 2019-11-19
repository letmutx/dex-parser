//! Dex `Type` and utilities
use std::clone::Clone;
use std::fmt;

use getset::{CopyGetters, Getters};

use crate::string::DexString;
use crate::uint;

/// Offset into the `TypeId`s section.
pub type TypeId = uint;

/// Represents a Java type. The type descriptor conforms to
/// the syntax described [here](https://source.android.com/devices/tech/dalvik/dex-format#typedescriptor)
#[derive(Debug, Getters, CopyGetters)]
pub struct Type {
    #[get_copy = "pub"]
    pub(crate) id: TypeId,
    /// The type descriptor string for this string.
    #[get = "pub"]
    pub(crate) type_descriptor: DexString,
}

impl Clone for Type {
    fn clone(&self) -> Self {
        Type {
            id: self.id,
            type_descriptor: self.type_descriptor.clone(),
        }
    }
}

impl PartialEq<Type> for Type {
    fn eq(&self, other: &Type) -> bool {
        self.id == other.id
    }
}

impl PartialEq<DexString> for Type {
    fn eq(&self, other: &DexString) -> bool {
        self.type_descriptor() == other
    }
}

impl PartialEq<str> for Type {
    fn eq(&self, other: &str) -> bool {
        self.type_descriptor() == other
    }
}

impl<'a> PartialEq<&'a str> for Type {
    fn eq(&self, other: &&'a str) -> bool {
        self.type_descriptor() == *other
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_descriptor)
    }
}
