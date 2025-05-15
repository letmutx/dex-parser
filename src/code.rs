//! Structures defining the contents of a `Method`'s code.
use scroll::{ctx, Pread, Sleb128, Uleb128};
use std::{fmt, ops::Deref};

use crate::jtype::TypeId;
use crate::string::StringId;
use crate::{
    encoded_item::EncodedCatchHandlers, error::Error, jtype::Type, string::DexString, uint, ulong,
    ushort,
};
use getset::{CopyGetters, Getters};

/// Debug Info of a method.
/// [Android docs](https://source.android.com/devices/tech/dalvik/dex-format#debug-info-item)
#[derive(Debug, Getters, CopyGetters)]
pub struct DebugInfoItem {
    /// Initial value for the state machines's line register.
    #[get_copy = "pub"]
    line_start: usize,
    /// Names of the incoming parameters.
    #[get = "pub"]
    parameter_names: Vec<Option<DexString>>,
    /// State machine bytecodes
    #[get = "pub"]
    bytecodes: Vec<DebugInfoBytecode>,
}

#[derive(Debug, PartialEq, Getters, CopyGetters)]
pub struct DebugInfoLocal {
    /// Register that will contain local
    #[get_copy = "pub"]
    register_num: u64,
    /// String index of the name
    #[get_copy = "pub"]
    name_idx: StringId,
    /// Type index of the type
    #[get_copy = "pub"]
    type_idx: TypeId,
    /// String index of the type signature
    #[get_copy = "pub"]
    sig_idx: Option<StringId>,
}

#[derive(Debug, PartialEq, Getters, CopyGetters)]
pub struct DebugInfoSpecial {
    // How many lines to move
    #[get_copy = "pub"]
    line_off: i64,
    // How many instructions to move
    #[get_copy = "pub"]
    address_off: u64,
}

#[derive(Debug, PartialEq)]
pub enum DebugInfoBytecode {
    /// Ends the debug info item
    EndSequence,
    /// Move to the next instruction
    AdvancePc(u64),
    /// Move to the next line
    AdvanceLine(i64),
    /// Creates a new variable
    StartLocal(DebugInfoLocal),
    /// Destroys a variable
    EndLocal(u64),
    /// Recreates a variable
    RestartLocal(u64),
    /// Ends a method prologue
    SetPrologueEnd,
    /// Begins a method prologue
    SetEpilogueBegin,
    /// Sets the file name
    SetFile(DexString),
    /// Moves to a new instruction and line and emit both
    Special(DebugInfoSpecial),
}

/// Code and Debug Info of a method.
#[derive(Getters, CopyGetters)]
pub struct CodeItem {
    /// The number of registers the method must use.
    #[get_copy = "pub"]
    registers_size: ushort,
    /// Line number and source file information.
    debug_info_item: Option<DebugInfoItem>,
    /// Number of words for incoming arguments to this method.
    #[get_copy = "pub"]
    ins_size: ushort,
    /// Number of words for outgoing arguments required for invocation.
    #[get_copy = "pub"]
    outs_size: ushort,
    /// Code instructions for this method.
    #[get = "pub"]
    insns: Vec<ushort>,
    /// Try, Exception handling information of this method.
    #[get = "pub"]
    tries: Tries,
}

impl CodeItem {
    /// Line number and source file information.
    pub fn debug_info_item(&self) -> Option<&DebugInfoItem> {
        self.debug_info_item.as_ref()
    }
}

impl fmt::Debug for CodeItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodeItem {{ registers_size: {}, debug_info: {}, ins_size: {}, outs_size: {}, tries: {} }}",
            self.registers_size, self.debug_info_item.is_some(), self.ins_size, self.outs_size, self.tries.len())
    }
}

/// Represents a Try-Catch block
#[derive(Pread, Clone, Copy, Debug, Getters, CopyGetters)]
pub(crate) struct TryItem {
    /// The instruction at which the try block starts.
    #[get_copy = "pub"]
    start_addr: uint,
    /// Number of instructions the try block covers.
    #[get_copy = "pub"]
    insn_count: ushort,
    /// Exception handler offset.
    #[get_copy = "pub"]
    handler_off: ushort,
}

#[derive(Debug, Clone)]
pub enum ExceptionType {
    /// The `Exception` class.
    BaseException,
    /// Sub-types of the `Exception` class.
    Ty(Type),
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct CatchHandler {
    /// Type of the exception handled by this handler.
    #[get = "pub"]
    pub(crate) exception: ExceptionType,
    /// Start address of the catch handler.
    #[get_copy = "pub"]
    pub(crate) addr: ulong,
}

/// Represents Try and catch blocks.
#[derive(Debug, Getters, CopyGetters)]
pub struct TryCatchHandlers {
    /// Start of the try block.
    #[get_copy = "pub"]
    start_addr: uint,
    /// Number of instructions covered by this try block.
    #[get_copy = "pub"]
    insn_count: ushort,
    /// List of catch handlers for this try block.
    #[get = "pub"]
    catch_handlers: Vec<CatchHandler>,
}

/// List of try-catch blocks found in this method.
#[derive(Debug, Default, Getters, CopyGetters)]
pub struct Tries {
    #[get = "pub"]
    try_catch_blocks: Vec<TryCatchHandlers>,
}

impl Deref for Tries {
    type Target = Vec<TryCatchHandlers>;

    fn deref(&self) -> &Self::Target {
        &self.try_catch_blocks
    }
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
        Ok((
            Self {
                try_catch_blocks: tries?,
            },
            *offset,
        ))
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
            let string_id = (Uleb128::read(source, offset)? as u32).overflowing_sub(1).0;
            parameter_names.push(if string_id != u32::from(crate::NO_INDEX) {
                Some(dex.get_string(string_id as uint)?)
            } else {
                None
            });
        }
        let mut bytecodes = Vec::new();
        loop {
            let opcode: u8 = source.gread(offset)?;

            let byte_code = match opcode {
                0x00 => DebugInfoBytecode::EndSequence,
                0x01 => DebugInfoBytecode::AdvancePc(Uleb128::read(source, offset)?),
                0x02 => DebugInfoBytecode::AdvanceLine(Sleb128::read(source, offset)?),
                0x03 => DebugInfoBytecode::StartLocal(DebugInfoLocal {
                    register_num: Uleb128::read(source, offset)?,
                    name_idx: (Uleb128::read(source, offset)? as StringId)
                        .overflowing_sub(1)
                        .0,
                    type_idx: (Uleb128::read(source, offset)? as TypeId)
                        .overflowing_sub(1)
                        .0,
                    sig_idx: None,
                }),
                0x04 => DebugInfoBytecode::StartLocal(DebugInfoLocal {
                    register_num: Uleb128::read(source, offset)?,
                    name_idx: (Uleb128::read(source, offset)? as StringId)
                        .overflowing_sub(1)
                        .0,
                    type_idx: (Uleb128::read(source, offset)? as TypeId)
                        .overflowing_sub(1)
                        .0,
                    sig_idx: Some(
                        (Uleb128::read(source, offset)? as StringId)
                            .overflowing_sub(1)
                            .0,
                    ),
                }),
                0x05 => DebugInfoBytecode::EndLocal(Uleb128::read(source, offset)?),
                0x06 => DebugInfoBytecode::RestartLocal(Uleb128::read(source, offset)?),
                0x07 => DebugInfoBytecode::SetPrologueEnd,
                0x08 => DebugInfoBytecode::SetEpilogueBegin,
                0x09 => DebugInfoBytecode::SetFile(
                    dex.get_string(
                        (Uleb128::read(source, offset)? as StringId)
                            .overflowing_sub(1)
                            .0 as uint,
                    )?,
                ),
                _ => {
                    const DBG_FIRST_SPECIAL: u64 = 0x0a; // the smallest special opcode
                    const DBG_LINE_BASE: i64 = -4; // the smallest line number increment
                    const DBG_LINE_RANGE: u64 = 15; // the number of line increments represented

                    let adjusted_opcode = opcode as u64 - DBG_FIRST_SPECIAL;

                    DebugInfoBytecode::Special(DebugInfoSpecial {
                        line_off: DBG_LINE_BASE + (adjusted_opcode % DBG_LINE_RANGE) as i64,
                        address_off: (adjusted_opcode / DBG_LINE_RANGE),
                    })
                }
            };

            bytecodes.push(byte_code);
            if bytecodes.last() == Some(&DebugInfoBytecode::EndSequence) {
                break;
            }
        }
        Ok((
            Self {
                line_start,
                parameter_names,
                bytecodes,
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
        let tries: Tries = if tries_size != 0 {
            source.gread_with(offset, (tries_size as usize, dex))?
        } else {
            Default::default()
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
