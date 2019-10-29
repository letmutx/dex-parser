#[macro_use]
extern crate scroll_derive;

use scroll;

pub use error::Error;

pub use crate::dex::Dex;
pub use crate::dex::DexBuilder;

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
mod source;
mod string;

const NO_INDEX: uint = 0xffff_ffff;

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
