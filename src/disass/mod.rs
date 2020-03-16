use crate::ushort;
use std::convert::TryInto;

mod opcodes;

trait InstGetter {
    fn length(&self) -> u32;
    fn a(&self, data: &[u8]) -> u64;
    fn b(&self, data: &[u8]) -> u64;
    fn c(&self, data: &[u8]) -> u64;
    fn d(&self, data: &[u8]) -> u64;
    fn e(&self, data: &[u8]) -> u64;
    fn f(&self, data: &[u8]) -> u64;
    fn g(&self, data: &[u8]) -> u64;
    fn h(&self, data: &[u8]) -> u64;
}

fn _b(data: &[u8], n: usize) -> u64 {
    (data[n] << (n * 8)).into()
}

fn read_l(data: &[u8]) -> u64 {
    (data[0] & 0x0f).into()
}

fn read_h(data: &[u8]) -> u64 {
    (data[0] >> 4).into()
}

fn read_2(data: &[u8]) -> u16 {
    (data[0] as u64 + _b(data, 1)).try_into().unwrap()
}

fn read_4(data: &[u8]) -> u32 {
    (data[0] as u64 + _b(data, 1) + _b(data, 2) + _b(data, 3))
        .try_into()
        .unwrap()
}

fn read_8(data: &[u8]) -> u64 {
    data[0] as u64
        + _b(data, 1)
        + _b(data, 2)
        + _b(data, 3)
        + _b(data, 4)
        + _b(data, 5)
        + _b(data, 6)
        + _b(data, 7)
}

impl InstGetter for GetterOp00 {
    fn length(&self) -> u32 {
        2
    }

    fn a(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00 can't get A");
    }

    fn b(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00 can't get B");
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00 can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00 can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00 can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00 can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00 can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00 can't get H");
    }
}

impl InstGetter for GetterOpAA {
    fn length(&self) -> u32 {
        2
    }

    fn a(&self, data: &[u8]) -> u64 {
        data[1].into()
    }

    fn b(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAA can't get B");
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAA can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAA can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAA can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAA can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAA can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAA can't get H");
    }
}

impl InstGetter for Getter10t {
    fn length(&self) -> u32 {
        2
    }

    fn a(&self, data: &[u8]) -> u64 {
        (data[1] as i8) as u64
    }

    fn b(&self, _data: &[u8]) -> u64 {
        panic!("Getter10t can't get B");
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("Getter10t can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("Getter10t can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("Getter10t can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("Getter10t can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("Getter10t can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("Getter10t can't get H");
    }
}

impl InstGetter for GetterOpBA {
    fn length(&self) -> u32 {
        2
    }

    fn a(&self, data: &[u8]) -> u64 {
        read_l(&data[1..])
    }

    fn b(&self, data: &[u8]) -> u64 {
        read_h(&data[1..])
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBA can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBA can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBA can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBA can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBA can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBA can't get H");
    }
}

impl InstGetter for GetterOp00AAAA {
    fn length(&self) -> u32 {
        4
    }

    fn a(&self, data: &[u8]) -> u64 {
        read_2(&data[2..]).into()
    }

    fn b(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAA can't get B");
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAA can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAA can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAA can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAA can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAA can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAA can't get H");
    }
}

impl InstGetter for Getter20t {
    fn length(&self) -> u32 {
        4
    }

    fn a(&self, data: &[u8]) -> u64 {
        (read_2(&data[2..]) as i16) as u64
    }

    fn b(&self, _data: &[u8]) -> u64 {
        panic!("Getter20t can't get B");
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("Getter20t can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("Getter20t can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("Getter20t can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("Getter20t can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("Getter20t can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("Getter20t can't get H");
    }
}

impl InstGetter for GetterOpAABBBB {
    fn length(&self) -> u32 {
        4
    }
    fn a(&self, data: &[u8]) -> u64 {
        data[1].into()
    }

    fn b(&self, data: &[u8]) -> u64 {
        read_2(&data[2..]).into()
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBB can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBB can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBB can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBB can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBB can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBB can't get H");
    }
}

impl InstGetter for GetterOpAACCBB {
    fn length(&self) -> u32 {
        4
    }
    fn a(&self, data: &[u8]) -> u64 {
        data[1].into()
    }

    fn b(&self, data: &[u8]) -> u64 {
        data[3].into()
    }

    fn c(&self, data: &[u8]) -> u64 {
        data[2].into()
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAACCBB can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAACCBB can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAACCBB can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAACCBB can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAACCBB can't get H");
    }
}

impl InstGetter for Getter21t {
    fn length(&self) -> u32 {
        4
    }
    fn a(&self, data: &[u8]) -> u64 {
        data[1].into()
    }
    fn b(&self, data: &[u8]) -> u64 {
        (read_2(&data[2..]) as i16) as u64
    }

    fn c(&self, data: &[u8]) -> u64 {
        data[2].into()
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("Getter21t can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("Getter21t can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("Getter21t can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("Getter21t can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("Getter21t can't get H");
    }
}

impl InstGetter for GetterOpBACCCC {
    fn length(&self) -> u32 {
        4
    }
    fn a(&self, data: &[u8]) -> u64 {
        read_l(&data[1..])
    }
    fn b(&self, data: &[u8]) -> u64 {
        read_h(&data[1..])
    }
    fn c(&self, data: &[u8]) -> u64 {
        read_2(&data[2..]).into()
    }
    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBACCCC can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBACCCC can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBACCCC can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBACCCC can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpBACCCC can't get H");
    }
}

impl InstGetter for Getter22t {
    fn length(&self) -> u32 {
        4
    }
    fn a(&self, data: &[u8]) -> u64 {
        read_l(&data[1..])
    }
    fn b(&self, data: &[u8]) -> u64 {
        read_h(&data[1..])
    }
    fn c(&self, data: &[u8]) -> u64 {
        (read_2(&data[2..]) as i16) as u64
    }
    fn d(&self, _data: &[u8]) -> u64 {
        panic!("Getter22t can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("Getter22t can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("Getter22t can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("Getter22t can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("Getter22t can't get H");
    }
}

impl InstGetter for Getter30t {
    fn length(&self) -> u32 {
        6
    }
    fn a(&self, data: &[u8]) -> u64 {
        (read_4(&data[2..]) as i32) as u64
    }
    fn b(&self, _data: &[u8]) -> u64 {
        panic!("Getter30t can't get B");
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("Getter30t can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("Getter30t can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("Getter30t can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("Getter30t can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("Getter30t can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("Getter30t can't get H");
    }
}

impl InstGetter for GetterOp00AAAAAAAA {
    fn length(&self) -> u32 {
        6
    }
    fn a(&self, data: &[u8]) -> u64 {
        read_4(&data[2..]).into()
    }
    fn b(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAAAAAA can't get B");
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAAAAAA can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAAAAAA can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAAAAAA can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAAAAAA can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAAAAAA can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAAAAAA can't get H");
    }
}

impl InstGetter for GetterOp00AAAABBBB {
    fn length(&self) -> u32 {
        6
    }
    fn a(&self, data: &[u8]) -> u64 {
        read_2(&data[2..]).into()
    }
    fn b(&self, data: &[u8]) -> u64 {
        read_2(&data[4..]).into()
    }
    fn c(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAABBBB can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAABBBB can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAABBBB can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAABBBB can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAABBBB can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOp00AAAABBBB can't get H");
    }
}

impl InstGetter for GetterOpAABBBBBBBB {
    fn length(&self) -> u32 {
        6
    }
    fn a(&self, data: &[u8]) -> u64 {
        data[1].into()
    }
    fn b(&self, data: &[u8]) -> u64 {
        read_4(&data[2..]).into()
    }
    fn c(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBBBBB can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBBBBB can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBBBBB can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBBBBB can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBBBBB can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBBBBB can't get H");
    }
}

impl InstGetter for GetterOpAABBBBCCCC {
    fn length(&self) -> u32 {
        6
    }
    fn a(&self, data: &[u8]) -> u64 {
        data[1].into()
    }
    fn b(&self, data: &[u8]) -> u64 {
        read_2(&data[2..]).into()
    }
    fn c(&self, data: &[u8]) -> u64 {
        read_2(&data[4..]).into()
    }
    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBCCCC can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBCCCC can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBCCCC can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBCCCC can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBCCCC can't get H");
    }
}

impl InstGetter for GetterOpAGBBBBDCFE {
    fn length(&self) -> u32 {
        6
    }
    fn a(&self, data: &[u8]) -> u64 {
        read_h(&data[1..])
    }
    fn b(&self, data: &[u8]) -> u64 {
        read_2(&data[2..]).into()
    }
    fn c(&self, data: &[u8]) -> u64 {
        read_l(&data[4..])
    }
    fn d(&self, data: &[u8]) -> u64 {
        read_h(&data[4..])
    }
    fn e(&self, data: &[u8]) -> u64 {
        read_l(&data[5..])
    }
    fn f(&self, data: &[u8]) -> u64 {
        read_h(&data[5..])
    }
    fn g(&self, data: &[u8]) -> u64 {
        read_l(&data[1..])
    }
    fn h(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAGBBBBDCFE can't get H");
    }
}

impl InstGetter for GetterOpAABBBBCCCCHHHH {
    fn length(&self) -> u32 {
        8
    }
    fn a(&self, data: &[u8]) -> u64 {
        data[1].into()
    }
    fn b(&self, data: &[u8]) -> u64 {
        read_2(&data[2..]).into()
    }
    fn c(&self, data: &[u8]) -> u64 {
        read_2(&data[4..]).into()
    }
    fn d(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBCCCCHHHH can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBCCCCHHHH can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBCCCCHHHH can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("GetterOpAABBBBCCCCHHHH can't get G");
    }
    fn h(&self, data: &[u8]) -> u64 {
        read_2(&data[6..]).into()
    }
}

impl InstGetter for GetterOpAGBBBBDCFEHHHH {
    fn length(&self) -> u32 {
        8
    }
    fn a(&self, data: &[u8]) -> u64 {
        read_h(&data[1..])
    }
    fn b(&self, data: &[u8]) -> u64 {
        read_2(&data[2..]).into()
    }
    fn c(&self, data: &[u8]) -> u64 {
        read_l(&data[4..])
    }
    fn d(&self, data: &[u8]) -> u64 {
        read_h(&data[4..])
    }
    fn e(&self, data: &[u8]) -> u64 {
        read_l(&data[5..])
    }
    fn f(&self, data: &[u8]) -> u64 {
        read_h(&data[5..])
    }
    fn g(&self, data: &[u8]) -> u64 {
        read_l(&data[1..])
    }
    fn h(&self, data: &[u8]) -> u64 {
        read_2(&data[6..]).into()
    }
}

impl InstGetter for GetterOpAABBBBBBBBBBBBBBBB {
    fn length(&self) -> u32 {
        10
    }
    fn a(&self, data: &[u8]) -> u64 {
        data[1].into()
    }
    fn b(&self, data: &[u8]) -> u64 {
        read_8(&data[2..])
    }

    fn c(&self, _data: &[u8]) -> u64 {
        panic!("OpAABBBBBBBBBBBBBBBB can't get C");
    }

    fn d(&self, _data: &[u8]) -> u64 {
        panic!("OpAABBBBBBBBBBBBBBBB can't get D");
    }

    fn e(&self, _data: &[u8]) -> u64 {
        panic!("OpAABBBBBBBBBBBBBBBB can't get E");
    }

    fn f(&self, _data: &[u8]) -> u64 {
        panic!("OpAABBBBBBBBBBBBBBBB can't get F");
    }

    fn g(&self, _data: &[u8]) -> u64 {
        panic!("OpAABBBBBBBBBBBBBBBB can't get G");
    }

    fn h(&self, _data: &[u8]) -> u64 {
        panic!("OpAABBBBBBBBBBBBBBBB can't get H");
    }
}

struct GetterOp00;
struct GetterOpAA;
struct Getter10t;
struct GetterOpBA;
struct GetterOp00AAAA;
struct Getter20t;
struct GetterOpAABBBB;
struct Getter21t;
struct GetterOpAACCBB;
struct GetterOpBACCCC;
struct Getter22t;
struct GetterOp00AAAAAAAA;
struct Getter30t;
struct GetterOp00AAAABBBB;
struct GetterOpAABBBBBBBB;
struct GetterOpAABBBBCCCC;
struct GetterOpAGBBBBDCFE;
struct GetterOpAABBBBCCCCHHHH;
struct GetterOpAGBBBBDCFEHHHH;
struct GetterOpAABBBBBBBBBBBBBBBB;

struct InstType {
    pub mnemonic: &'static str,
    pub syntax: &'static str,
    pub get: &'static dyn InstGetter,
}

const INSTTYPES: [InstType; 256] = include!("insn.in");

pub struct Inst<'a> {
    pub bytes: &'a [u8],
}

macro_rules! table {
    ($s:expr) => {
        &INSTTYPES[$s.op()].get
    };
}

impl Inst<'_> {
    pub fn op(&self) -> usize {
        self.bytes[0].into()
    }

     pub fn length(&self) -> usize {
        if self.op() == 0 {
            if self.bytes[1] == 0 {
                return 2;
            } else if self.bytes[1] == 1 {
                // packed-switch-payload
                let size: usize = (((self.bytes[3] as usize) << 8) + (self.bytes[2] as usize))
                    .try_into()
                    .unwrap();
                return 8 + 4 * size;
            } else if self.bytes[1] == 2 {
                // sparse-switch-payload
                let size: usize = (((self.bytes[3] as usize) << 8) + (self.bytes[2] as usize))
                    .try_into()
                    .unwrap();
                return 4 + 8 * size;
            } else if self.bytes[1] == 3 {
                // fill-array-data-payload
                let w = ((self.bytes[3] as usize) << 8) + (self.bytes[2] as usize);
                let n = ((self.bytes[7] as usize) << 24)
                    + ((self.bytes[6] as usize) << 16)
                    + ((self.bytes[5] as usize) << 8)
                    + (self.bytes[4] as usize);
                let mut len: usize = (8 + w * n).try_into().unwrap();
                if len % 2 == 1 {
                    len += 1;
                }
                return len;
            }

            panic!("Unexpected NOP type {:x}", self.bytes[1]);
        }

        return table!(self).length();
    }


    pub fn get_a(&self) -> u64 {
        table!(self).a(self.bytes)
    }

    pub fn get_b(&self) -> u64 {
        table!(self).b(self.bytes)
    }

    pub fn get_c(&self) -> u64 {
        table!(self).c(self.bytes)
    }

    pub fn get_d(&self) -> u64 {
        table!(self).d(self.bytes)
    }

    pub fn get_e(&self) -> u64 {
        table!(self).e(self.bytes)
    }

    pub fn get_f(&self) -> u64 {
        table!(self).f(self.bytes)
    }

    pub fn get_g(&self) -> u64 {
        table!(self).g(self.bytes)
    }

    pub fn get_h(&self) -> u64 {
        table!(self).h(self.bytes)
    }

   pub fn is_const(&self) -> bool {
        Const4 <= self.op() && self.op() <= ConstClass
    }

    pub fn is_const_string(&self) -> bool {
        self.op() == ConstString || self.op() == ConstStringJumbo
    }

    pub fn is_invoke(&self) -> bool {
        if InvokeVirtual <= self.op() && self.op() <= InvokeInterface {
            return true;
        }
        if InvokeVirtual_range <= self.op() && self.op() <= InvokeInterface_range {
            return true;
        }
        if self.op() == InvokePolymorphic || self.op() == InvokePolymorphic_range {
            return true;
        }
        return false;
    }

    pub fn is_read_field(&self) -> bool {
        if Iget <= self.op() && self.op() <= IgetShort {
            return true;
        }
        if Sget <= self.op() && self.op() <= SgetShort {
            return true;
        }
        return false;
    }

    pub fn is_return(&self) -> bool {
        ReturnVoid <= self.op() && self.op() <= ReturnObject
    }

    pub fn is_throw(&self) -> bool {
        self.op() == Throw
    }

    pub fn is_goto(&self) -> bool {
        Goto <= self.op() && self.op() <= Goto_32
    }

    pub fn is_branch(&self) -> bool {
        IfEq <= self.op() && self.op() <= IfLez
    }

    pub fn is_switch(&self) -> bool {
        self.op() == PackedSwitch || self.op() == SparseSwitch
    }

    pub fn string_idx(&self) -> i32 {
        self.get_b() as i32
    }

    pub fn invoke_target(&self) -> i32 {
        self.get_b() as i32
    }

    pub fn field(&self) -> i32 {
        if self.op() < Sget {
            self.get_c().try_into().unwrap()
        } else {
            self.get_b().try_into().unwrap()
        }
    }
}

impl fmt::Debug for Inst<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "lenght={} data={:?}",
            self.length(),
            &self.bytes[..self.length()]
        ))
    }
}

impl fmt::Display for Inst<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", INSTTYPES[self.op()].mnemonic)
    }
}

pub struct InstIterator<'a> {
    pub bytes: &'a [u8],
    index: usize,
    length: usize,
    current_inst: Inst<'a>,
}

impl InstIterator<'_> {
    fn new(bytes: &[u8], length: usize) -> InstIterator {
        InstIterator {
            bytes: bytes,
            index: 0,
            length: length,
            current_inst: Inst { bytes: bytes },
        }
    }
}

impl<'a> Iterator for InstIterator<'a> {
    type Item = Inst<'a>;

    fn next(&mut self) -> Option<Inst<'a>> {
        if self.index < self.length {
            let i = Inst {
                bytes: &self.bytes[self.index..],
            };
            self.index += i.length();
            Some(i)
        } else {
            None
        }
    }
}


