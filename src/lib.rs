#[macro_use]
extern crate scroll_derive;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;

use scroll;

pub use error::Error;

pub use crate::dex::Dex;
pub use crate::dex::DexReader;

#[macro_use]
mod utils;
mod annotation;
mod cache;
mod class;
mod code;
mod dex;
mod encoded_item;
mod encoded_value;
mod error;
mod field;
mod jtype;
mod method;
mod search;
mod source;
mod string;

bitflags! {
    pub struct ClassAccessFlags: uint {
        const PUBLIC = 0x1;
        const PRIVATE = 0x2;
        const PROTECTED = 0x4;
        const STATIC = 0x8;
        const FINAL = 0x10;
        const INTERFACE = 0x200;
        const ABSTRACT = 0x400;
        const SYNTHETIC = 0x1000;
        const ANNOTATION = 0x2000;
        const ENUM = 0x4000;
    }
}

bitflags! {
    pub struct FieldAccessFlags: ulong {
        const PUBLIC = 0x1;
        const PRIVATE = 0x2;
        const PROTECTED = 0x4;
        const STATIC = 0x8;
        const FINAL = 0x10;
        const VOLATILE = 0x40;
        const TRANSIENT = 0x80;
        const SYNTHETIC = 0x1000;
        const ANNOTATION = 0x2000;
        const ENUM = 0x4000;
    }
}

bitflags! {
    pub struct MethodAccessFlags: ulong {
        const PUBLIC = 0x1;
        const PRIVATE = 0x2;
        const PROTECTED = 0x4;
        const STATIC = 0x8;
        const FINAL = 0x10;
        const SYNCHRONIZED = 0x20;
        const BRIDGE = 0x40;
        const VARARGS = 0x80;
        const NATIVE = 0x100;
        const ABSTRACT = 0x400;
        const STRICT = 0x800;
        const SYNTHETIC = 0x1000;
        const CONSTRUCTOR = 0x10000;
        const DECLARED_SYNCHRONIZED = 0x20000;
    }
}

const NO_INDEX: uint = 0xffff_ffff;
const ENDIAN_CONSTANT: (ubyte, ubyte, ubyte, ubyte) = (0x12, 0x34, 0x56, 0x78);
const REVERSE_ENDIAN_CONSTANT: (ubyte, ubyte, ubyte, ubyte) = (0x78, 0x56, 0x34, 0x12);

#[allow(non_camel_case_types)]
pub type byte = i8;
#[allow(non_camel_case_types)]
pub type uint = u32;
#[allow(non_camel_case_types)]
pub type int = i32;
#[allow(non_camel_case_types)]
pub type ushort = u16;
#[allow(non_camel_case_types)]
pub type short = i16;
#[allow(non_camel_case_types)]
pub type ubyte = u8;
#[allow(non_camel_case_types)]
pub type ulong = u64;
#[allow(non_camel_case_types)]
pub type long = i64;

pub type Result<T> = std::result::Result<T, error::Error>;

// ref. https://source.android.com/devices/tech/dalvik/dex-format

pub type Endian = scroll::Endian;
