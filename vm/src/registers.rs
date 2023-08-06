use core::fmt;

use crate::abi::{Layout, Ty};
use crate::access::RegAccess;
use crate::capability::{Address, Capability, TaggedCapability};
use crate::exception::Exception;
use crate::int::{UAddr, UGran, UGRAN_SIZE};
use crate::mem::TagController;

#[derive(Debug)]
pub struct Registers {
    regs: [Capability; Self::COUNT as _],
}

impl Registers {
    pub const COUNT: u8 = 32;

    pub const fn new() -> Self {
        Self {
            regs: [Capability::INVALID; Self::COUNT as _],
        }
    }

    pub fn read(&self, tags: &TagController, reg: u8) -> Result<TaggedCapability, Exception> {
        let gran = self.read_data(reg)?;
        let valid = if reg == Register::Zero as _ {
            false
        } else {
            tags.read_reg(reg).unwrap()
        };
        Ok(TaggedCapability::new(Capability::from_ugran(gran), valid))
    }

    pub fn read_data(&self, reg: u8) -> Result<UGran, Exception> {
        let access = RegAccess {
            reg,
            len: Register::LAYOUT.size,
        };
        access.check_reg()?;
        debug_assert!(access.check_len().is_ok());
        if reg == Register::Zero as _ {
            return Ok(0);
        }
        let idx = Self::reg_to_idx(reg);
        Ok(self.regs[idx].to_ugran())
    }

    pub fn read_ty<T: Ty>(&self, tags: &TagController, reg: u8) -> Result<T, Exception> {
        let layout = T::LAYOUT;
        let access = RegAccess {
            reg,
            len: layout.size,
        };
        let data = self.read_data(reg)?.to_le_bytes();
        access.check_len()?;
        T::read(
            &data[..access.len as usize],
            Address(0),
            tags.reg(reg).unwrap(),
        )
    }

    pub fn write(
        &mut self,
        tags: &mut TagController,
        reg: u8,
        cap: TaggedCapability,
    ) -> Result<(), Exception> {
        let access = RegAccess {
            reg,
            len: Register::LAYOUT.size,
        };
        access.check_reg()?;
        debug_assert!(access.check_len().is_ok());
        let idx = Self::reg_to_idx(reg);
        self.regs[idx] = cap.capability();
        tags.write_reg(reg, cap.is_valid()).unwrap();
        Ok(())
    }

    pub fn write_data(
        &mut self,
        tags: &mut TagController,
        reg: u8,
        val: UGran,
    ) -> Result<(), Exception> {
        self.write(tags, reg, TaggedCapability::from_ugran(val))
    }

    pub fn write_ty<T: Ty>(
        &mut self,
        tags: &mut TagController,
        reg: u8,
        val: T,
    ) -> Result<(), Exception> {
        let layout = T::LAYOUT;
        let mut data = [0; UGRAN_SIZE as _];
        let access = RegAccess {
            reg,
            len: layout.size,
        };
        access.check()?;
        val.write(
            &mut data[..access.len as usize],
            Address(0),
            tags.reg_mut(reg).unwrap(),
        )?;
        self.regs[Self::reg_to_idx(reg)] = Capability::from_ugran(UGran::from_le_bytes(data));
        Ok(())
    }

    pub const fn is_reg_valid(reg: u8) -> bool {
        (reg & Self::MASK) == reg
    }

    pub const fn is_len_valid(len: UAddr) -> bool {
        len <= Register::LAYOUT.size
    }
}

impl Registers {
    const MASK: u8 = 0b0001_1111;

    const fn reg_to_idx(reg: u8) -> usize {
        (reg & Self::MASK) as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Register {
    Zero,
    Pc,
    Ra,
    Sp,
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,

    // reserved, but currently used as magic place to find parent allocator
    Z0,
}

impl Register {
    pub const LAYOUT: Layout = UGran::LAYOUT;
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Zero => "zero",
            Self::Pc => "pc",
            Self::Ra => "ra",
            Self::Sp => "sp",
            Self::T0 => "t0",
            Self::T1 => "t1",
            Self::T2 => "t2",
            Self::T3 => "t3",
            Self::T4 => "t4",
            Self::T5 => "t5",
            Self::T6 => "t6",
            Self::A0 => "a0",
            Self::A1 => "a1",
            Self::A2 => "a2",
            Self::A3 => "a3",
            Self::A4 => "a4",
            Self::A5 => "a5",
            Self::A6 => "a6",
            Self::A7 => "a7",
            Self::S0 => "s0",
            Self::S1 => "s1",
            Self::S2 => "s2",
            Self::S3 => "s3",
            Self::S4 => "s4",
            Self::S5 => "s5",
            Self::S6 => "s6",
            Self::S7 => "s7",
            Self::S8 => "s8",
            Self::S9 => "s9",
            Self::S10 => "s10",
            Self::S11 => "s11",
            Self::Z0 => "z0",
        };
        f.write_str(s)
    }
}
