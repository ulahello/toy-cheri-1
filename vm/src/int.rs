use core::mem;

use crate::abi::{Align, Layout, Ty};
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::mem::Memory;

pub type UGran = u128;
pub type SGran = i128;
pub type UAddr = u64;
pub type SAddr = i64;

pub const UADDR_SIZE: u8 = mem::size_of::<UAddr>() as _;
pub const UGRAN_SIZE: u8 = mem::size_of::<UGran>() as _;

pub const UNINIT: UAddr = UAddr::from_le_bytes([UNINIT_BYTE; UADDR_SIZE as _]);
pub const UNINIT_BYTE: u8 = 0x55;

pub const fn gran_sign(u: UGran) -> SGran {
    SGran::from_le_bytes(u.to_le_bytes())
}

pub const fn gran_unsign(s: SGran) -> UGran {
    UGran::from_le_bytes(s.to_le_bytes())
}

pub const fn addr_sign(u: UAddr) -> SAddr {
    SAddr::from_le_bytes(u.to_le_bytes())
}

pub const fn addr_unsign(s: SAddr) -> UAddr {
    UAddr::from_le_bytes(s.to_le_bytes())
}

// TODO: Ty impls for ints are duplicated

impl Ty for UAddr {
    const LAYOUT: Layout = Layout {
        size: UADDR_SIZE as _,
        align: Align::new(1).unwrap(),
    };

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let slice = mem.read_raw(src, Self::LAYOUT)?;
        Ok(Self::from_le_bytes(slice.try_into().unwrap()))
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        mem.write_raw(dst, Self::LAYOUT.align, &self.to_le_bytes())
    }
}

impl Ty for UGran {
    const LAYOUT: Layout = Layout {
        size: UGRAN_SIZE as _,
        align: Align::new(1).unwrap(),
    };

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let slice = mem.read_raw(src, Self::LAYOUT)?;
        Ok(Self::from_le_bytes(slice.try_into().unwrap()))
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        mem.write_raw(dst, Self::LAYOUT.align, &self.to_le_bytes())
    }
}

impl Ty for u8 {
    const LAYOUT: Layout = Layout {
        size: mem::size_of::<u8>() as _,
        align: Align::new(1).unwrap(),
    };

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let slice = mem.read_raw(src, Self::LAYOUT)?;
        Ok(Self::from_le_bytes(slice.try_into().unwrap()))
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        mem.write_raw(dst, Self::LAYOUT.align, &self.to_le_bytes())
    }
}

impl Ty for u16 {
    const LAYOUT: Layout = Layout {
        size: mem::size_of::<u16>() as _,
        align: Align::new(1).unwrap(),
    };

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let slice = mem.read_raw(src, Self::LAYOUT)?;
        Ok(Self::from_le_bytes(slice.try_into().unwrap()))
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        mem.write_raw(dst, Self::LAYOUT.align, &self.to_le_bytes())
    }
}

impl Ty for u32 {
    const LAYOUT: Layout = Layout {
        size: mem::size_of::<u32>() as _,
        align: Align::new(1).unwrap(),
    };

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let slice = mem.read_raw(src, Self::LAYOUT)?;
        Ok(Self::from_le_bytes(slice.try_into().unwrap()))
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        mem.write_raw(dst, Self::LAYOUT.align, &self.to_le_bytes())
    }
}
