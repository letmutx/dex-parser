use scroll::Pread;

use crate::encoded_item::EncodedCatchHandlerList;
use crate::jtype::Type;
use crate::uint;
use crate::ulong;
use crate::ushort;
use scroll::ctx;

#[derive(Debug)]
pub struct CodeItem {
    registers_size: ushort,
    debug_info_off: uint,
    ins_size: ushort,
    outs_size: ushort,
    insns: Vec<ushort>,
    tries: Option<Vec<TryCatchHandlers>>,
}

#[derive(Pread, Clone, Copy, Debug)]
pub(crate) struct TryItem {
    start_addr: uint,
    insn_count: ushort,
    handler_off: ushort,
}

#[derive(Debug, Clone)]
pub enum ExceptionType {
    BaseException,
    Ty(Type),
}

#[derive(Debug, Clone)]
pub struct CatchHandler {
    pub(crate) exception: ExceptionType,
    pub(crate) addr: ulong,
}

#[derive(Debug)]
pub struct TryCatchHandlers {
    start_addr: uint,
    insn_count: ushort,
    catch_handlers: Vec<CatchHandler>,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for CodeItem
where
    S: AsRef<[u8]>,
{
    type Error = crate::error::Error;
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
        let insns_size: uint = source.gread_with(offset, endian)?;
        let insns: Vec<ushort> = read_vec!(source, offset, insns_size, endian);
        if insns_size % 2 != 0 && tries_size != 0 {
            source.gread_with::<ushort>(offset, endian)?;
        }
        let tries = if tries_size != 0 {
            let tries: Vec<TryItem> = read_vec!(source, offset, tries_size, endian);
            let encoded_catch_handler_list: EncodedCatchHandlerList =
                source.gread_with(offset, dex)?;
            let tries: super::Result<Vec<_>> = tries
                .into_iter()
                .map(|c| {
                    let (_, encoded_catch_handler) = encoded_catch_handler_list
                        .iter()
                        .find(|p| p.0 == c.handler_off as usize)
                        .ok_or_else(|| {
                            crate::error::Error::InvalidId(format!(
                                "Invalid catch handler: {}",
                                c.handler_off
                            ))
                        })?;

                    Ok(TryCatchHandlers {
                        start_addr: c.start_addr,
                        insn_count: c.insn_count,
                        catch_handlers: encoded_catch_handler.handlers.clone(),
                    })
                })
                .collect();
            Some(tries?)
        } else {
            None
        };

        Ok((
            Self {
                registers_size,
                debug_info_off,
                ins_size,
                outs_size,
                insns,
                tries,
            },
            *offset,
        ))
    }
}
