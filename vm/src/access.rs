use core::fmt;

use crate::abi::Align;
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::int::UAddr;
use crate::registers::Registers;

#[derive(Clone, Copy, Debug)]
pub struct MemAccess {
    pub tcap: TaggedCapability,
    pub len: Option<UAddr>, // None indicates overflow
    pub align: Align,
    pub kind: MemAccessKind,
}

impl MemAccess {
    pub const fn is_bounded(&self) -> bool {
        if let Some(len) = self.len {
            self.tcap.is_bounded_with_len(len)
        } else {
            false
        }
    }

    pub const fn perms_grant(&self) -> bool {
        self.tcap.perms().grants_access(self.kind)
    }

    pub const fn is_aligned(&self) -> bool {
        self.tcap.addr().is_aligned_to(self.align)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MemAccessKind {
    Read,
    Write,
    Execute,
}

impl fmt::Display for MemAccessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Execute => "execute",
        };
        f.write_str(name)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RegAccess {
    pub reg: u8,
    pub len: UAddr,
}

impl RegAccess {
    pub const fn is_reg_valid(&self) -> bool {
        Registers::is_reg_valid(self.reg)
    }

    pub const fn is_len_valid(&self) -> bool {
        Registers::is_len_valid(self.len)
    }

    pub const fn check_reg(self) -> Result<(), Exception> {
        if self.is_reg_valid() {
            Ok(())
        } else {
            Err(Exception::InvalidRegAccess { access: self })
        }
    }

    pub const fn check_len(self) -> Result<(), Exception> {
        if self.is_len_valid() {
            Ok(())
        } else {
            Err(Exception::InvalidRegAccess { access: self })
        }
    }

    pub fn check(&self) -> Result<(), Exception> {
        self.check_reg()?;
        self.check_len()?;
        Ok(())
    }
}
