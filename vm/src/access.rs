use core::fmt;

use crate::abi::Align;
use crate::capability::TaggedCapability;
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
        let span_start = self.tcap.addr();
        if let Some(len) = self.len {
            let span_end = self.tcap.addr().add(len.saturating_sub(1));
            self.tcap.is_addr_bounded(span_start)
                && (len == 0 || self.tcap.is_addr_bounded(span_end))
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
}

impl RegAccess {
    pub const fn is_reg_valid(&self) -> bool {
        Registers::is_reg_valid(self.reg)
    }
}
