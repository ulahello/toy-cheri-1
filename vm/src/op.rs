use core::fmt;

use crate::abi::{Align, Fields, Layout, Ty};
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::mem::Memory;

// TODO: document which operations operate on capabilities
// TODO: document that branch instructions truncate to Address sized offsets

/* TODOOO: manipulation of cababilities */
// informally based on riscv but this is not by definition so could change anytime
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OpKind {
    /// No-op.
    Nop = 0,

    /// Load immediate value `op2` into register `op1`.
    LoadI,

    /// Load 8-bit value from capability at register `op2` and zero-extend
    /// before storing it in register `op1`.
    LoadU8,

    /// Load 16-bit value from capability at register `op2` and zero-extend
    /// before storing it in register `op1`.
    LoadU16,

    /// Load 32-bit value from capability at register `op2` and zero-extend
    /// before storing it in register `op1`.
    LoadU32,

    /// Load 64-bit value from capability at register `op2` and zero-extend
    /// before storing it in register `op1`.
    LoadU64,

    /// Load 128-bit value from capability at register `op2` into register
    /// `op1`.
    LoadU128,

    /// Load capability from capability at register `op2` into register `op1`.
    LoadC,

    /// Store 8-bit value from the low bits of register `op2` to capability at
    /// register `op1`.
    Store8,

    /// Store 16-bit value from the low bits of register `op2` to capability at
    /// register `op1`.
    Store16,

    /// Store 32-bit value from the low bits of register `op2` to capability at
    /// register `op1`.
    Store32,

    /// Store 64-bit value from the low bits of register `op2` to capability at
    /// register `op1`.
    Store64,

    /// Store 128-bit value from the low bits of register `op2` to capability at
    /// register `op1`.
    Store128,

    /// Store capability from register `op2` to capability at register `op1`.
    StoreC,

    /// Add immediate `op3` to register `op2` and store the result in register
    /// `op1`. Arithmetic overflow is ignored.
    AddI,

    /// Add registers `op3` to `op2` and store the result in register `op1`.
    /// Arithmetic overflow is ignored.
    Add,

    /// Subtract registers `op3` from `op2` and store the result in register
    /// `op1`. Arithmetic overflow is ignored.
    Sub,

    /// Place the value 1 in register `op1` if register `op2` is less than
    /// immediate `op3` when both are treated as signed numbers, else 0 is
    /// written to `op1`.
    SltsI,

    /// Place the value 1 in register `op1` if register `op2` is less than
    /// immediate `op3` when both are treated as unsigned numbers, else 0 is
    /// written to `op1`.
    SltuI,

    /// Place the value 1 in register `op1` if register `op2` is less than
    /// register `op3` when both are treated as signed numbers, else 0 is
    /// written to `op1`.
    Slts,

    /// Place the value 1 in register `op1` if register `op2` is less than
    /// register `op3` when both are treated as unsigned numbers, else 0 is
    /// written to `op1`.
    Sltu,

    /// Perform bitwise XOR on register `op2` and immediate `op3` and store the
    /// result in register `op1`.
    XorI,

    /// Perform bitwise XOR on registers `op2` and `op3` and store the result in
    /// register `op1`.
    Xor,

    /// Perform bitwise OR on register `op2` and immediate `op3` and store the
    /// result in register `op1`.
    OrI,

    /// Perform bitwise OR on registers `op2` and `op3` and store the result in
    /// register `op1`.
    Or,

    /// Perform bitwise AND on register `op2` and immediate `op3` and store the
    /// result in register `op1`.
    AndI,

    /// Perform bitwise AND on registers `op2` and `op3` and store the result in
    /// register `op1`.
    And,

    /// Perform logical left shift on the value in register `op2` by the shift
    /// amount held in immediate `op3` and store the result in register `op1`.
    SllI,

    /// Perform logical left shift on the value in register `op2` by the shift
    /// amount held in register `op3` and store the result in register `op1`.
    Sll,

    /// Perform logical right shift on the value in register `op2` by the shift
    /// amount held in immediate `op3` and store the result in register `op1`.
    SrlI,

    /// Perform logical right shift on the value in register `op2` by the shift
    /// amount held in register `op3` and store the result in register `op1`.
    Srl,

    /// Perform arithmetic right shift on the value in register `op2` by the
    /// shift amount held in immediate `op3` and store the result in register
    /// `op1`.
    SraI,

    /// Perform arithmetic right shift on the value in register `op2` by the
    /// shift amount held in register `op3` and store the result in register
    /// `op1`.
    Sra,

    /// Offset the program counter by immediate `op2` and store the return
    /// address in register `op1`.
    ///
    /// Offset is computed in multiples of `Op::LAYOUT.size`.
    Jal,

    /// Offset the program counter by the sum of immediate `op3` and register
    /// `op2` and store the return address in register `op1`.
    ///
    /// Offset is computed in multiples of `Op::LAYOUT.size`.
    Jalr,

    /// Offset the program counter by immediate `op3` if the values of registers
    /// `op1` and `op2` are equal.
    ///
    /// Offset is computed in multiples of `Op::LAYOUT.size`.
    Beq,

    /// Offset the program counter by immediate `op3` if the values of registers
    /// `op1` and `op2` are not equal.
    ///
    /// Offset is computed in multiples of `Op::LAYOUT.size`.
    Bne,

    /// Offset the program counter by immediate `op3` if the value of registers
    /// `op1` is less `op2`, using signed comparison.
    ///
    /// Offset is computed in multiples of `Op::LAYOUT.size`.
    Blts,

    /// Offset the program counter by immediate `op3` if the value of registers
    /// `op1` is greater than or equal to `op2`, using signed comparison.
    ///
    /// Offset is computed in multiples of `Op::LAYOUT.size`.
    Bges,

    /// Offset the program counter by immediate `op3` if value of registers
    /// `op1` is less than `op2`, using unsigned comparison.
    ///
    /// Offset is computed in multiples of `Op::LAYOUT.size`.
    Bltu,

    /// Offset the program counter by immediate `op3` if the value of registers
    /// `op1` is greater than or equal to `op2`, using unsigned comparison.
    ///
    /// Offset is computed in multiples of `Op::LAYOUT.size`.
    Bgeu,

    /// Perform a system call. The [kind](crate::syscall::SyscallKind) is
    /// determined by the value in register `a0`.
    Syscall,
}

impl OpKind {
    pub const MAX_OPERANDS: usize = 3;

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn from_byte(byte: u8) -> Result<Self, Exception> {
        match byte {
            0 => Ok(Self::Nop),
            1 => Ok(Self::LoadI),
            2 => Ok(Self::LoadU8),
            3 => Ok(Self::LoadU16),
            4 => Ok(Self::LoadU32),
            5 => Ok(Self::LoadU64),
            6 => Ok(Self::LoadU128),
            7 => Ok(Self::LoadC),
            8 => Ok(Self::Store8),
            9 => Ok(Self::Store16),
            10 => Ok(Self::Store32),
            11 => Ok(Self::Store64),
            12 => Ok(Self::Store128),
            13 => Ok(Self::StoreC),
            14 => Ok(Self::AddI),
            15 => Ok(Self::Add),
            16 => Ok(Self::Sub),
            17 => Ok(Self::SltsI),
            18 => Ok(Self::SltuI),
            19 => Ok(Self::Slts),
            20 => Ok(Self::Sltu),
            21 => Ok(Self::XorI),
            22 => Ok(Self::Xor),
            23 => Ok(Self::OrI),
            24 => Ok(Self::Or),
            25 => Ok(Self::AndI),
            26 => Ok(Self::And),
            27 => Ok(Self::SllI),
            28 => Ok(Self::Sll),
            29 => Ok(Self::SrlI),
            30 => Ok(Self::Srl),
            31 => Ok(Self::SraI),
            32 => Ok(Self::Sra),
            33 => Ok(Self::Jal),
            34 => Ok(Self::Jalr),
            35 => Ok(Self::Beq),
            36 => Ok(Self::Bne),
            37 => Ok(Self::Blts),
            38 => Ok(Self::Bges),
            39 => Ok(Self::Bltu),
            40 => Ok(Self::Bgeu),
            41 => Ok(Self::Syscall),
            _ => Err(Exception::InvalidOpKind { byte }),
        }
    }

    pub const fn operand_count(self) -> u8 {
        match self {
            Self::Nop => 0,
            Self::LoadI => 2,
            Self::LoadU8 => 2,
            Self::LoadU16 => 2,
            Self::LoadU32 => 2,
            Self::LoadU64 => 2,
            Self::LoadU128 => 2,
            Self::LoadC => 2,
            Self::Store8 => 2,
            Self::Store16 => 2,
            Self::Store32 => 2,
            Self::Store64 => 2,
            Self::Store128 => 2,
            Self::StoreC => 2,
            Self::AddI => 3,
            Self::Add => 3,
            Self::Sub => 3,
            Self::SltsI => 3,
            Self::SltuI => 3,
            Self::Slts => 3,
            Self::Sltu => 3,
            Self::XorI => 3,
            Self::Xor => 3,
            Self::OrI => 3,
            Self::Or => 3,
            Self::AndI => 3,
            Self::And => 3,
            Self::SllI => 3,
            Self::Sll => 3,
            Self::SrlI => 3,
            Self::Srl => 3,
            Self::SraI => 3,
            Self::Sra => 3,
            Self::Jal => 2,
            Self::Jalr => 3,
            Self::Beq => 3,
            Self::Bne => 3,
            Self::Blts => 3,
            Self::Bges => 3,
            Self::Bltu => 3,
            Self::Bgeu => 3,
            Self::Syscall => 0,
        }
    }

    pub const fn type_signature(self) -> [Option<OperandType>; 3] {
        const fn sig<const N: usize>(
            op: OpKind,
            sig: [OperandType; N],
        ) -> [Option<OperandType>; OpKind::MAX_OPERANDS] {
            assert!(N <= OpKind::MAX_OPERANDS);
            if sig.len() != op.operand_count() as _ {
                panic!("signature must have correct operand cound");
            }
            let mut out = [None; 3];

            let mut idx = 0;
            while idx < sig.len() {
                out[idx] = Some(sig[idx]);
                idx += 1;
            }
            out
        }

        use OperandType::*;

        match self {
            Self::Nop => sig(self, []),
            Self::LoadI => sig(self, [Register, Immediate]),
            Self::LoadU8 => sig(self, [Register, Register]),
            Self::LoadU16 => sig(self, [Register, Register]),
            Self::LoadU32 => sig(self, [Register, Register]),
            Self::LoadU64 => sig(self, [Register, Register]),
            Self::LoadU128 => sig(self, [Register, Register]),
            Self::LoadC => sig(self, [Register, Register]),
            Self::Store8 => sig(self, [Register, Register]),
            Self::Store16 => sig(self, [Register, Register]),
            Self::Store32 => sig(self, [Register, Register]),
            Self::Store64 => sig(self, [Register, Register]),
            Self::Store128 => sig(self, [Register, Register]),
            Self::StoreC => sig(self, [Register, Register]),
            Self::AddI => sig(self, [Register, Register, Immediate]),
            Self::Add => sig(self, [Register, Register, Register]),
            Self::Sub => sig(self, [Register, Register, Register]),
            Self::SltsI => sig(self, [Register, Register, Immediate]),
            Self::SltuI => sig(self, [Register, Register, Immediate]),
            Self::Slts => sig(self, [Register, Register, Register]),
            Self::Sltu => sig(self, [Register, Register, Register]),
            Self::XorI => sig(self, [Register, Register, Immediate]),
            Self::Xor => sig(self, [Register, Register, Register]),
            Self::OrI => sig(self, [Register, Register, Immediate]),
            Self::Or => sig(self, [Register, Register, Register]),
            Self::AndI => sig(self, [Register, Register, Immediate]),
            Self::And => sig(self, [Register, Register, Register]),
            Self::SllI => sig(self, [Register, Register, Immediate]),
            Self::Sll => sig(self, [Register, Register, Register]),
            Self::SrlI => sig(self, [Register, Register, Immediate]),
            Self::Srl => sig(self, [Register, Register, Register]),
            Self::SraI => sig(self, [Register, Register, Immediate]),
            Self::Sra => sig(self, [Register, Register, Register]),
            Self::Jal => sig(self, [Register, Immediate]),
            Self::Jalr => sig(self, [Register, Register, Immediate]),
            Self::Beq => sig(self, [Register, Register, Immediate]),
            Self::Bne => sig(self, [Register, Register, Immediate]),
            Self::Blts => sig(self, [Register, Register, Immediate]),
            Self::Bges => sig(self, [Register, Register, Immediate]),
            Self::Bltu => sig(self, [Register, Register, Immediate]),
            Self::Bgeu => sig(self, [Register, Register, Immediate]),
            Self::Syscall => sig(self, []),
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

impl fmt::Display for OpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Nop => "nop",
            Self::LoadI => "loadi",
            Self::LoadU8 => "loadu8",
            Self::LoadU16 => "loadu16",
            Self::LoadU32 => "loadu32",
            Self::LoadU64 => "loadu64",
            Self::LoadU128 => "loadu128",
            Self::LoadC => "loadc",
            Self::Store8 => "store8",
            Self::Store16 => "store16",
            Self::Store32 => "store32",
            Self::Store64 => "store64",
            Self::Store128 => "store128",
            Self::StoreC => "storec",
            Self::AddI => "addi",
            Self::Add => "add",
            Self::Sub => "sub",
            Self::SltsI => "sltsi",
            Self::SltuI => "sltui",
            Self::Slts => "slts",
            Self::Sltu => "sltu",
            Self::XorI => "xori",
            Self::Xor => "xor",
            Self::OrI => "ori",
            Self::Or => "or",
            Self::AndI => "andi",
            Self::And => "and",
            Self::SllI => "slli",
            Self::Sll => "sll",
            Self::SrlI => "srli",
            Self::Srl => "srl",
            Self::SraI => "srai",
            Self::Sra => "sra",
            Self::Jal => "jal",
            Self::Jalr => "jalr",
            Self::Beq => "beq",
            Self::Bne => "bne",
            Self::Blts => "blts",
            Self::Bges => "bges",
            Self::Bltu => "bltu",
            Self::Bgeu => "bgeu",
            Self::Syscall => "syscall",
        };
        f.write_str(s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandType {
    Register,
    Immediate,
}

impl fmt::Display for OperandType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Register => "register",
            Self::Immediate => "immediate",
        };
        f.write_str(s)
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
