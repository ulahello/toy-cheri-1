use core::fmt;

use crate::abi::{Align, Fields, Layout, Ty};
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::mem::Memory;

/* TODOOO: turing complete memory manipulation */
/* TODOOO: manipulation of cababilities */
// informally based on riscv but this is not by definition so could change anytime
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    pub const fn operand_count(self) -> u8 {
        match self {
            Self::Nop => 0,
            Self::LoadI => 2,
            Self::Syscall => 0,
        }
    }
}

impl Ty for OpKind {
    const LAYOUT: Layout = Layout {
        size: 1,
        align: Align::new(1).unwrap(),
    };

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        Self::from_byte(mem.read(src)?)
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        mem.write(dst, self.to_byte())
    }
}

/* TODOO: we cant know addresses of everything before we load into mem. encoded
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
     * memory efficient because not all operands are always needed */

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
            op1: TaggedCapability::from_ugran(dst as _), // register destination
            op2: imm,                                    // immediate value
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

impl Op {
    const FIELDS: &'static [Layout] = &[
        OpKind::LAYOUT,
        TaggedCapability::LAYOUT,
        TaggedCapability::LAYOUT,
        TaggedCapability::LAYOUT,
    ];
}

impl Ty for Op {
    const LAYOUT: Layout = Fields::layout(Self::FIELDS);

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let mut fields = Fields::new(src, Self::FIELDS);
        let kind_c = fields.next().unwrap();
        let op1_c = fields.next().unwrap();
        let op2_c = fields.next().unwrap();
        let op3_c = fields.next().unwrap();
        Ok(Self {
            kind: OpKind::read_from_mem(kind_c, mem)?,
            op1: TaggedCapability::read_from_mem(op1_c, mem)?,
            op2: TaggedCapability::read_from_mem(op2_c, mem)?,
            op3: TaggedCapability::read_from_mem(op3_c, mem)?,
        })
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        let mut fields = Fields::new(dst, Self::FIELDS);
        let kind_c = fields.next().unwrap();
        let op1_c = fields.next().unwrap();
        let op2_c = fields.next().unwrap();
        let op3_c = fields.next().unwrap();
        self.kind.write_to_mem(kind_c, mem)?;
        self.op1.write_to_mem(op1_c, mem)?;
        self.op2.write_to_mem(op2_c, mem)?;
        self.op3.write_to_mem(op3_c, mem)?;
        Ok(())
    }
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut dbg = f.debug_struct("Op");
        dbg.field("kind", &self.kind);
        for (i, op) in [self.op1, self.op2, self.op3].into_iter().enumerate() {
            let i = i as u8;
            if i < self.kind.operand_count() {
                dbg.field(&format!("op{i}", i = i + 1), &op);
            }
        }
        dbg.finish()
    }
}

impl PartialEq for Op {
    fn eq(&self, other: &Op) -> bool {
        // unused fields are ignored
        let ops = [
            (&self.op1, &other.op1),
            (&self.op2, &other.op2),
            (&self.op3, &other.op3),
        ];
        let mut acc = self.kind == other.kind;
        for (me, you) in ops.into_iter().take(self.kind.operand_count().into()) {
            if !acc {
                return false;
            }
            acc &= me == you;
        }
        acc
    }
}

impl Eq for Op {}
