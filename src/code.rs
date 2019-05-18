use scroll::Pread;
use scroll::Uleb128;

use crate::encoded_item::EncodedCatchHandler;
use crate::encoded_item::EncodedCatchHandlerList;
use crate::jtype::Type;

pub struct CodeItem {
    registers_size: u16,
    debug_info_off: u32,
    ins_size: u16,
    outs_size: u16,
    insns: Vec<u16>,
    tries: Vec<TryCatchHandlers>,
}

#[derive(Pread, Clone, Copy)]
pub(crate) struct TryItem {
    start_addr: u32,
    insn_count: u16,
    handler_off: u16,
}

pub enum ExceptionType {
    BaseException,
    Ty(Type),
}

pub struct CatchHandler {
    exception: ExceptionType,
    addr: u64,
}

pub struct TryCatchHandlers {
    start_addr: u32,
    insn_count: u16,
    catch_handlers: Vec<CatchHandler>,
}

impl CodeItem {
    pub(crate) fn try_from_dex<S: AsRef<[u8]>>(
        dex: &super::Dex<S>,
        offset: u64,
    ) -> super::Result<Self> {
        let source = dex.source.as_ref().as_ref();
        let offset = &mut (offset as usize);
        let endian = dex.get_endian();
        let registers_size: u16 = source.gread_with(offset, endian)?;
        let ins_size = source.gread_with(offset, endian)?;
        let outs_size = source.gread_with(offset, endian)?;
        let tries_size: u16 = source.gread_with(offset, endian)?;
        let debug_info_off = source.gread_with(offset, endian)?;
        let insns_size: u32 = source.gread_with(offset, endian)?;
        let mut insns: Vec<u16> = Vec::with_capacity(insns_size as usize);
        source.gread_inout_with(offset, &mut insns, endian)?;
        if insns_size % 2 != 0 && tries_size != 0 {
            source.gread_inout_with(offset, &mut [0, 0], endian)?;
        }
        let mut tries: Vec<TryItem> = Vec::with_capacity(tries_size as usize);
        source.gread_inout_with(offset, &mut tries, endian)?;
        let encoded_handler_size = Uleb128::read(source, offset)?;
        let mut encoded_catch_handlers: Vec<EncodedCatchHandlerList> =
            Vec::with_capacity(encoded_handler_size as usize);
        source.gread_inout_with(offset, &mut encoded_catch_handlers, ())?;
        let tries: super::Result<Vec<_>> = tries
            .into_iter()
            .map(|c| {
                let handlers: super::Result<Vec<_>> = encoded_catch_handlers
                    .iter()
                    .filter_map(|p| p.iter().find(|p| p.0 == c.handler_off as usize))
                    .map(|e| match e.1 {
                        EncodedCatchHandler::CatchAll(addr) => Ok(CatchHandler {
                            exception: ExceptionType::BaseException,
                            addr: addr as u64,
                        }),
                        EncodedCatchHandler::Type(type_addr_pair) => Ok(CatchHandler {
                            exception: ExceptionType::Ty(dex.get_type(type_addr_pair.type_id)?),
                            addr: type_addr_pair.addr,
                        }),
                    })
                    .collect();

                match handlers {
                    Ok(ref handlers) if handlers.len() == 0 => Err(crate::error::Error::InvalidId(
                        "Invalid catch handler offset".to_string(),
                    )),
                    Ok(handlers) => Ok(TryCatchHandlers {
                        start_addr: c.start_addr,
                        insn_count: c.insn_count,
                        catch_handlers: handlers,
                    }),
                    Err(e) => Err(e),
                }
            })
            .collect();
        let tries = tries?;

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
