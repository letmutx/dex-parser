use scroll::ctx;
use scroll::{Pread, Uleb128};

use crate::cache::Ref;
use crate::encoded_item::EncodedCatchHandlers;
use crate::error::Error;
use crate::jtype::Type;
use crate::string::JString;
use crate::uint;
use crate::ulong;
use crate::ushort;

/// Debug Info of a method.
/// https://source.android.com/devices/tech/dalvik/dex-format#debug-info-item
#[derive(Debug)]
pub struct DebugInfoItem {
    line_start: usize,
    parameter_names: Vec<Option<Ref<JString>>>,
}

/// Code and Debug Info of a method.
#[derive(Debug)]
pub struct CodeItem {
    /// The number of registers the method must use.
    registers_size: ushort,
    /// Line number and source file information.
    debug_info_item: Option<DebugInfoItem>,
    /// Number of words for incoming arguments to this method.
    ins_size: ushort,
    /// Number of words for outgoing arguments required for invocation.
    outs_size: ushort,
    /// Code instructions for this method.
    insns: Vec<ushort>,
    /// Try, Exception handling information of this method.
    tries: Option<Tries>,
}

/// Represents a Try-Catch block
#[derive(Pread, Clone, Copy, Debug)]
pub(crate) struct TryItem {
    /// The instruction at which the try block starts.
    start_addr: uint,
    /// Number of instructions the try block covers.
    insn_count: ushort,
    /// Exception handler offset.
    handler_off: ushort,
}

#[derive(Debug, Clone)]
pub enum ExceptionType {
    /// The `Exception` class.
    BaseException,
    /// Sub-types of the `Exception` class.
    Ty(Type),
}

#[derive(Debug, Clone)]
pub struct CatchHandler {
    /// Type of the exception handled by this handler.
    pub(crate) exception: ExceptionType,
    /// Start address of the catch handler.
    pub(crate) addr: ulong,
}

/// Represents Try and catch blocks.
#[derive(Debug)]
pub struct TryCatchHandlers {
    /// Start of the try block.
    start_addr: uint,
    /// Number of instructions covered by this try block.
    insn_count: ushort,
    /// List fo catch handlers for this try block.
    catch_handlers: Vec<CatchHandler>,
}

/// List of try-catch blocks found in this method.
#[derive(Debug)]
pub struct Tries {
    inner: Vec<TryCatchHandlers>,
}

impl<'a, S> ctx::TryFromCtx<'a, (usize, &super::Dex<S>)> for Tries
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(
        source: &'a [u8],
        (tries_size, dex): (usize, &super::Dex<S>),
    ) -> Result<(Self, Self::Size), Self::Error> {
        let offset = &mut 0;
        let endian = dex.get_endian();
        let tries: Vec<TryItem> = try_gread_vec_with!(source, offset, tries_size, endian);
        let encoded_catch_handlers: EncodedCatchHandlers = source.gread_with(offset, dex)?;
        let tries: super::Result<Vec<_>> = tries
            .into_iter()
            .map(|c| {
                let encoded_handler =
                    encoded_catch_handlers.find(c.handler_off).ok_or_else(|| {
                        Error::InvalidId(format!("Invalid catch handler: {}", c.handler_off))
                    })?;
                Ok(TryCatchHandlers {
                    start_addr: c.start_addr,
                    insn_count: c.insn_count,
                    catch_handlers: encoded_handler.handlers(),
                })
            })
            .collect();
        Ok((Self { inner: tries? }, *offset))
    }
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for DebugInfoItem
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(
        source: &'a [u8],
        dex: &super::Dex<S>,
    ) -> Result<(Self, Self::Size), Self::Error> {
        let offset = &mut 0;
        let line_start = Uleb128::read(source, offset)? as usize;
        let parameters_size = Uleb128::read(source, offset)?;
        let mut parameter_names = Vec::with_capacity(parameters_size as usize);
        for _ in 0..parameters_size {
            let string_id = Uleb128::read(source, offset)? + 1;
            parameter_names.push(if string_id != u64::from(crate::NO_INDEX) {
                Some(dex.get_string(string_id as uint)?)
            } else {
                None
            });
        }
        Ok((
            Self {
                line_start,
                parameter_names,
            },
            *offset,
        ))
    }
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for CodeItem
where
    S: AsRef<[u8]>,
{
    type Error = Error;
    type Size = usize;

    fn try_from_ctx(
        source: &'a [u8],
        dex: &super::Dex<S>,
    ) -> Result<(Self, Self::Size), Self::Error> {
        let offset = &mut 0;
        let endian = dex.get_endian();
        let registers_size: ushort = source.gread_with(offset, endian)?;
        let ins_size = source.gread_with(offset, endian)?;
        let outs_size = source.gread_with(offset, endian)?;
        let tries_size: ushort = source.gread_with(offset, endian)?;
        let debug_info_off = source.gread_with(offset, endian)?;
        let debug_info_item = if debug_info_off != 0 {
            Some(dex.get_debug_info_item(debug_info_off)?)
        } else {
            None
        };
        let insns_size: uint = source.gread_with(offset, endian)?;
        let insns: Vec<ushort> = try_gread_vec_with!(source, offset, insns_size, endian);
        if insns_size % 2 != 0 && tries_size != 0 {
            source.gread_with::<ushort>(offset, endian)?;
        }
        let tries: Option<Tries> = if tries_size != 0 {
            Some(source.gread_with(offset, (tries_size as usize, dex))?)
        } else {
            None
        };
        Ok((
            Self {
                registers_size,
                debug_info_item,
                ins_size,
                outs_size,
                insns,
                tries,
            },
            *offset,
        ))
    }
}
