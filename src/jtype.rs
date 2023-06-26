//! Dex `Type` and utilities
use std::{clone::Clone, fmt};

use getset::{CopyGetters, Getters};

use crate::{string::DexString, uint};

/// Dex representation of a boolean type
pub const BOOLEAN: &'static str = "Z";
/// Dex representation of a byte type
pub const BYTE: &'static str = "B";
/// Dex representation of a short type
pub const SHORT: &'static str = "S";
/// Dex representation of a char type
pub const CHAR: &'static str = "C";
/// Dex representation of an integer type
pub const INT: &'static str = "I";
/// Dex representation of a long type
pub const LONG: &'static str = "J";
/// Dex representation of a float type
pub const FLOAT: &'static str = "F";
/// Dex representation of a double type
pub const DOUBLE: &'static str = "D";
/// Dex representation of a void type
pub const VOID: &'static str = "V";

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

macro_rules! gen_is_type_method {
    ($func_name: ident, $descriptor: ident, $doc: literal) => {
        #[doc = $doc]
        pub fn $func_name(&self) -> bool {
            self.type_descriptor == $descriptor
        }
    }
}

impl Type {
    /// Returns `true` if the type is primitive
    pub fn is_primitive(&self) -> bool {
        self.is_bool()
            || self.is_byte()
            || self.is_short()
            || self.is_char()
            || self.is_int()
            || self.is_long()
            || self.is_float()
            || self.is_double()
            || self.is_void()
    }

    /// Returns `true` if the type is an array or a class
    pub fn is_reference(&self) -> bool {
        self.is_array() || self.is_class()
    }

    /// Returns `true` if the type is a class
    pub fn is_class(&self) -> bool {
        self.type_descriptor.starts_with("L")
    }

    /// Returns `true` if the type is an array
    pub fn is_array(&self) -> bool {
        self.type_descriptor.starts_with("[")
    }

    /// If the type represents an array, get it's dimensions,
    /// otherwise returns `None`
    pub fn array_dimensions(&self) -> Option<usize> {
        if self.is_array() {
            Some(
                self.type_descriptor
                    .chars()
                    .take_while(|c| *c == '[')
                    .count(),
            )
        } else {
            None
        }
    }

    /// Returns the Java representation of the `Type`
    pub fn to_java_type(&self) -> String {
        to_java_type(&*self.type_descriptor)
    }

    gen_is_type_method!(is_bool, BOOLEAN, "Returns `true` if the type is a boolean");
    gen_is_type_method!(is_byte, BYTE, "Returns `true` if the type is a byte");
    gen_is_type_method!(is_short, SHORT, "Returns `true` if the type is a short");
    gen_is_type_method!(is_char, CHAR, "Returns `true` if the type is a char");
    gen_is_type_method!(is_int, INT, "Returns `true` if the type is an integer");
    gen_is_type_method!(is_long, LONG, "Returns `true` if the type is a long");
    gen_is_type_method!(is_float, FLOAT, "Returns `true` if the type is a float");
    gen_is_type_method!(is_double, DOUBLE, "Returns `true` if the type is a double");
    gen_is_type_method!(is_void, VOID, "Returns `true` if the type is void");
}

fn to_java_type(s: &str) -> String {
    match s {
        BOOLEAN => "boolean".to_string(),
        BYTE => "byte".to_string(),
        SHORT => "short".to_string(),
        CHAR => "char".to_string(),
        INT => "int".to_string(),
        LONG => "long".to_string(),
        FLOAT => "float".to_string(),
        DOUBLE => "double".to_string(),
        VOID => "void".to_string(),
        s if s.starts_with('L') => s[1..].replace('/', ".").replace(';', ""),
        s if s.starts_with('[') => {
            let d = s.chars().take_while(|c| *c == '[').count();
            let mut base_type = to_java_type(&s[d..]);
            base_type.push_str(&"[]".repeat(d));
            base_type
        }
        _ => unreachable!(),
    }
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_to_java_type() {
        use super::to_java_type;
        assert_eq!(to_java_type(super::BOOLEAN), "boolean");
        assert_eq!(to_java_type(super::BYTE), "byte");
        assert_eq!(to_java_type(super::SHORT), "short");
        assert_eq!(to_java_type(super::CHAR), "char");
        assert_eq!(to_java_type(super::INT), "int");
        assert_eq!(to_java_type(super::LONG), "long");
        assert_eq!(to_java_type(super::FLOAT), "float");
        assert_eq!(to_java_type(super::DOUBLE), "double");
        assert_eq!(to_java_type(super::VOID), "void");
        assert_eq!(to_java_type("Ljava/lang/String;"), "java.lang.String");
        assert_eq!(to_java_type("[Ljava/lang/String;"), "java.lang.String[]");
        assert_eq!(to_java_type("[[Ljava/lang/String;"), "java.lang.String[][]");
    }
}
