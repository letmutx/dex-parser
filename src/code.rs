use scroll::Pread;

use crate::encoded_item::EncodedCatchHandlerList;
use crate::encoded_item::Handler;
use crate::jtype::Type;
use crate::uint;

#[derive(Debug)]
pub struct CodeItem {
    registers_size: u16,
    debug_info_off: uint,
    ins_size: u16,
    outs_size: u16,
    insns: Vec<u16>,
    tries: Option<Vec<TryCatchHandlers>>,
}

#[derive(Pread, Clone, Copy, Debug)]
pub(crate) struct TryItem {
    start_addr: uint,
    insn_count: u16,
    handler_off: u16,
}

#[derive(Debug)]
pub enum ExceptionType {
    BaseException,
    Ty(Type),
}

#[derive(Debug)]
pub struct CatchHandler {
    exception: ExceptionType,
    addr: u64,
}

#[derive(Debug)]
pub struct TryCatchHandlers {
    start_addr: uint,
    insn_count: u16,
    catch_handlers: Vec<CatchHandler>,
}

impl CodeItem {
    pub(crate) fn try_from_dex<S: AsRef<[u8]>>(
        dex: &super::Dex<S>,
        offset: u64,
    ) -> super::Result<Self> {
        let source = dex.source.as_ref();
        let mut offset = offset as usize;
        let offset = &mut offset;
        let endian = dex.get_endian();
        let registers_size: u16 = source.gread_with(offset, endian)?;
        let ins_size = source.gread_with(offset, endian)?;
        let outs_size = source.gread_with(offset, endian)?;
        let tries_size: u16 = source.gread_with(offset, endian)?;
        let debug_info_off = source.gread_with(offset, endian)?;
        let insns_size: uint = source.gread_with(offset, endian)?;
        let mut insns: Vec<u16> = Vec::with_capacity(insns_size as usize);
        for _ in 0..insns_size {
            insns.push(source.gread_with(offset, endian)?);
        }
        if insns_size % 2 != 0 && tries_size != 0 {
            source.gread_with::<u16>(offset, endian)?;
        }
        let tries = if tries_size != 0 {
            let mut tries: Vec<TryItem> = Vec::with_capacity(tries_size as usize);
            for _ in 0..tries_size {
                tries.push(source.gread_with(offset, endian)?);
            }
            let encoded_catch_handler_list: EncodedCatchHandlerList = source.gread(offset)?;
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
                    let catch_handlers: super::Result<Vec<_>> = encoded_catch_handler
                        .handlers
                        .iter()
                        .map(|handler| match handler {
                            Handler::CatchAll(addr) => Ok(CatchHandler {
                                exception: ExceptionType::BaseException,
                                addr: *addr as u64,
                            }),
                            Handler::Type(type_addr_pair) => Ok(CatchHandler {
                                exception: ExceptionType::Ty(dex.get_type(type_addr_pair.type_id)?),
                                addr: type_addr_pair.addr,
                            }),
                        })
                        .collect();

                    Ok(TryCatchHandlers {
                        start_addr: c.start_addr,
                        insn_count: c.insn_count,
                        catch_handlers: catch_handlers?,
                    })
                })
                .collect();
            Some(tries?)
        } else {
            None
        };

        Ok(Self {
            registers_size,
            debug_info_off,
            ins_size,
            outs_size,
            insns,
            tries,
        })
    }
}
