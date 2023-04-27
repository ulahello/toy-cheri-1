use core::fmt;

use crate::capability::{Capability, TaggedCapability};
use crate::exception::Exception;

/* TODO: turing complete memory manipulation */
/* TODO: manipulation of cababilities */
// informally based on riscv but this is not by definition so could change anytime
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum OpKind {
    /// No-op.
    Nop = 0,

    /// Load immediate value `op2` into register `op1`.
    LoadI,

    /// Perform a system call. The [kind](crate::syscall::SyscallKind) is
    /// determined by the value in register `a0`.
    Syscall,
}

impl OpKind {
    pub const SIZE: u8 = 1;

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn from_byte(byte: u8) -> Result<Self, Exception> {
        match byte {
            0 => Ok(Self::Nop),
            1 => Ok(Self::LoadI),
            2 => Ok(Self::Syscall),
            _ => Err(Exception::InvalidOpKind { byte }),
        }
    }

    pub const fn arg_count(self) -> u8 {
        match self {
            Self::Nop => 0,
            Self::LoadI => 2,
            Self::Syscall => 0,
        }
    }
}

/* TODO: we cant know addresses of everything before we load into mem. encoded
 * ops cant be tagged. their validity must be rebuilt from some sort of root
 * capability passed to the program. */
#[derive(Clone, Copy)]
pub struct Op {
    pub kind: OpKind,
    pub op1: TaggedCapability,
    pub op2: TaggedCapability,
    pub op3: TaggedCapability,
}

impl Op {
    /* TODO: currently implemented as constant size, but variable size is more
     * memory efficient because not all args are always needed */
    pub const SIZE: u8 = OpKind::SIZE + Capability::SIZE * 3;

    pub const fn nop() -> Self {
        Self {
            kind: OpKind::Nop,
            op1: TaggedCapability::INVALID,
            op2: TaggedCapability::INVALID,
            op3: TaggedCapability::INVALID,
        }
    }

    pub const fn loadi(dst: u8, imm: TaggedCapability) -> Self {
        Self {
            kind: OpKind::LoadI,
            op1: TaggedCapability {
                capa: Capability::from_ugran(dst as _),
                valid: false,
            }, // register destination
            op2: imm, // immediate value
            op3: TaggedCapability::INVALID,
        }
    }

    pub const fn syscall() -> Self {
        Self {
            kind: OpKind::Syscall,
            op1: TaggedCapability::INVALID,
            op2: TaggedCapability::INVALID,
            op3: TaggedCapability::INVALID,
        }
    }
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut dbg = f.debug_struct("Op");
        dbg.field("kind", &self.kind);
        for (i, op) in [self.op1, self.op2, self.op3].into_iter().enumerate() {
            let i = i as u8;
            if i < self.kind.arg_count() {
                dbg.field(&format!("op{i}", i = i + 1), &op);
            }
        }
        dbg.finish()
    }
}
