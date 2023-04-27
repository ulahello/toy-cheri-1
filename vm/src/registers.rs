use crate::access::RegAccess;
use crate::capability::{Capability, TaggedCapability};
use crate::exception::Exception;
use crate::mem::TagController;

#[derive(Debug)]
pub struct Registers {
    regs: [Capability; 32],
}

impl Registers {
    pub const COUNT: u8 = 32;

    pub const fn new() -> Self {
        Self {
            regs: [Capability::INVALID; Self::COUNT as _],
        }
    }

    pub fn read(&self, tags: &TagController, reg: u8) -> Result<TaggedCapability, Exception> {
        if Self::is_reg_valid(reg) {
            if reg == Register::Zero as _ {
                return Ok(TaggedCapability {
                    capa: Capability::from_ugran(0),
                    valid: false,
                });
            }
            let idx = Self::reg_to_idx(reg);
            Ok(TaggedCapability {
                capa: self.regs[idx],
                valid: tags.read_reg(reg).unwrap(),
            })
        } else {
            Err(Exception::InvalidRegAccess {
                access: RegAccess { reg },
            })
        }
    }

    pub fn write(
        &mut self,
        tags: &mut TagController,
        reg: u8,
        val: TaggedCapability,
    ) -> Result<(), Exception> {
        if Self::is_reg_valid(reg) {
            let idx = Self::reg_to_idx(reg);
            self.regs[idx] = val.capability();
            tags.write_reg(reg, val.is_valid()).unwrap();
            Ok(())
        } else {
            Err(Exception::InvalidRegAccess {
                access: RegAccess { reg },
            })
        }
    }

    pub const fn is_reg_valid(reg: u8) -> bool {
        (reg & Self::MASK) == reg
    }
}

impl Registers {
    const MASK: u8 = 0b0001_1111;

    const fn reg_to_idx(reg: u8) -> usize {
        (reg & Self::MASK) as usize
    }
}

#[derive(Clone, Copy, Debug)]
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

    // reserved
    Z0,
}
