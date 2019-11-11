// Silence warnings in error module for now
#![allow(bare_trait_objects)]

#[macro_use]
extern crate scroll_derive;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;

extern crate getset;

use scroll;

pub use error::Error;

pub use crate::dex::Dex;
pub use crate::dex::DexReader;

#[macro_use]
mod utils;
pub mod annotation;
mod cache;
pub mod class;
mod code;
mod dex;
mod encoded_item;
pub mod encoded_value;
mod error;
pub mod field;
pub mod jtype;
pub mod method;
mod search;
mod source;
mod string;

/// The constant NO_INDEX is used to indicate that an index value is absent.
pub const NO_INDEX: uint = 0xffff_ffff;
const ENDIAN_CONSTANT: (ubyte, ubyte, ubyte, ubyte) = (0x12, 0x34, 0x56, 0x78);
const REVERSE_ENDIAN_CONSTANT: (ubyte, ubyte, ubyte, ubyte) = (0x78, 0x56, 0x34, 0x12);

/// 8-bit signed int
#[allow(non_camel_case_types)]
pub type byte = i8;
/// 32-bit unsigned int
#[allow(non_camel_case_types)]
pub type uint = u32;
/// 32-bit signed int
#[allow(non_camel_case_types)]
pub type int = i32;
/// 16-bit unsigned int
#[allow(non_camel_case_types)]
pub type ushort = u16;
/// 16-bit signed int
#[allow(non_camel_case_types)]
pub type short = i16;
/// 8-bit unsigned int
#[allow(non_camel_case_types)]
pub type ubyte = u8;
/// 64-bit unsigned int
#[allow(non_camel_case_types)]
pub type ulong = u64;
/// 64-bit signed int
#[allow(non_camel_case_types)]
pub type long = i64;

/// A `Result` of `T` or an error of `error::Error`
pub type Result<T> = std::result::Result<T, error::Error>;

// ref. https://source.android.com/devices/tech/dalvik/dex-format

/// The endianness of bytes.
pub type Endian = scroll::Endian;
