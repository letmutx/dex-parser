//! Dex `Type` and utilities
use std::clone::Clone;
use std::fmt;

use getset::{CopyGetters, Getters};

use crate::string::DexString;
use crate::uint;

pub type TypeId = uint;

// TODO: add new function
/// Represents a Java type. The type descriptor conforms to
/// https://source.android.com/devices/tech/dalvik/dex-format#typedescriptor
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

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_descriptor)
    }
}
