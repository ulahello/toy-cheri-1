use bitvec::slice::BitSlice;

use core::fmt;

use crate::abi::{self, Align, Layout, StructMut, StructRef, Ty};
use crate::capability::{Address, TaggedCapability};
use crate::exception::Exception;

// informally based on riscv but this is not by definition so could change anytime
#[deny(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
/// Enumeration over all operations.
pub enum OpKind {
    /// No-op.
    Nop = 0,

    /// Load the address value from the capability at register `op2` and store
    /// it in register `op1`.
    CGetAddr,

    /// Assign the address value at register `op2` to the capability in register
    /// `op1`.
    CSetAddr,

    /// Load the start and end bound values from the capability at register
    /// `op3` and store them in registers `op1` and `op2`, respectively.
    CGetBound,

    /// Assign the start and end bound values at registers `op2` and `op3`,
    /// respectively, to the capability in register `op1`. If the new bounds are
    /// wider than the old bounds, the capability will be invalidated.
    CSetBound,

    /// Load the permissions bit field from the capability at register `op2` and
    /// store it in register `op1`.
    CGetPerm,

    /// Assign the permissions bit field at register `op2` to the capability in
    /// register `op1`. If the new permissions are more permissive than the old
    /// permissions, the capability will be invalidated.
    CSetPerm,

    /// Place the value 1 in register `op1` if the capability at register `op2`
    /// is valid, else place 0.
    CGetValid,

    /// Load capability from register `op2` to register `op1`.
    Cpy,

    /// Load immediate value `op2` into register `op1`.
    LoadI,

    /// Load 8-bit value from memory at register `op2` and zero-extend before
    /// storing it in register `op1`.
    LoadU8,

    /// Load 16-bit value from memory at register `op2` and zero-extend before
    /// storing it in register `op1`.
    LoadU16,

    /// Load 32-bit value from memory at register `op2` and zero-extend before
    /// storing it in register `op1`.
    LoadU32,

    /// Load 64-bit value from memory at register `op2` and zero-extend before
    /// storing it in register `op1`.
    LoadU64,

    /// Load 128-bit value from memory at register `op2` into register `op1`.
    LoadU128,

    /// Load capability from memory at register `op2` into register `op1`.
    LoadC,

    /// Store 8-bit value from the low bits of register `op2` to memory at
    /// register `op1`.
    Store8,

    /// Store 16-bit value from the low bits of register `op2` to memory at
    /// register `op1`.
    Store16,

    /// Store 32-bit value from the low bits of register `op2` to memory at
    /// register `op1`.
    Store32,

    /// Store 64-bit value from the low bits of register `op2` to memory at
    /// register `op1`.
    Store64,

    /// Store 128-bit value from the low bits of register `op2` to memory at
    /// register `op1`.
    Store128,

    /// Store capability from register `op2` to memory at register `op1`.
    StoreC,

    /// Add immediate `op3` to register `op2` and store the result in register
    /// `op1`.
    ///
    /// Values wrap upon arithmetic overflow.
    AddI,

    /// Add registers `op3` to `op2` and store the result in register `op1`.
    ///
    /// Values wrap upon arithmetic overflow.
    Add,

    /// Subtract registers `op3` from `op2` and store the result in register
    /// `op1`.
    ///
    /// Values wrap upon arithmetic overflow.
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

    /// Offset the program counter address by immediate `op2` and store the
    /// return address in register `op1`.
    ///
    /// All computations leading to the offset operate on `SAddr` values.
    Jal,

    /// Set the program counter address to the sum of `SAddr` immediate `op3`
    /// and `UAddr` register `op2` and store the return address in register
    /// `op1`.
    Jalr,

    /// Offset the program counter address by immediate `op3` if the values of
    /// registers `op1` and `op2` are equal.
    ///
    /// All computations leading to the offset operate on `SAddr` values.
    Beq,

    /// Offset the program counter address by immediate `op3` if the values of
    /// registers `op1` and `op2` are not equal.
    ///
    /// All computations leading to the offset wrap upon overflow and operate
    /// on `SAddr` values.
    Bne,

    /// Offset the program counter address by immediate `op3` if the value of
    /// registers `op1` is less `op2`, using signed comparison.
    ///
    /// All computations leading to the offset wrap upon overflow and operate
    /// on `SAddr` values.
    Blts,

    /// Offset the program counter address by immediate `op3` if the value of
    /// registers `op1` is greater than or equal to `op2`, using signed
    /// comparison.
    ///
    /// All computations leading to the offset wrap upon overflow and operate
    /// on `SAddr` values.
    Bges,

    /// Offset the program counter address by immediate `op3` if value of
    /// registers `op1` is less than `op2`, using unsigned comparison.
    ///
    /// All computations leading to the offset wrap upon overflow and operate
    /// on `SAddr` values.
    Bltu,

    /// Offset the program counter address by immediate `op3` if the value of
    /// registers `op1` is greater than or equal to `op2`, using unsigned
    /// comparison.
    ///
    /// All computations leading to the offset wrap upon overflow and operate
    /// on `SAddr` values.
    Bgeu,

    /// Perform a system call. The [kind](crate::syscall::SyscallKind) is
    /// determined by the value in register `a2`.
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
            1 => Ok(Self::CGetAddr),
            2 => Ok(Self::CSetAddr),
            3 => Ok(Self::CGetBound),
            4 => Ok(Self::CSetBound),
            5 => Ok(Self::CGetPerm),
            6 => Ok(Self::CSetPerm),
            7 => Ok(Self::CGetValid),
            8 => Ok(Self::Cpy),
            9 => Ok(Self::LoadI),
            10 => Ok(Self::LoadU8),
            11 => Ok(Self::LoadU16),
            12 => Ok(Self::LoadU32),
            13 => Ok(Self::LoadU64),
            14 => Ok(Self::LoadU128),
            15 => Ok(Self::LoadC),
            16 => Ok(Self::Store8),
            17 => Ok(Self::Store16),
            18 => Ok(Self::Store32),
            19 => Ok(Self::Store64),
            20 => Ok(Self::Store128),
            21 => Ok(Self::StoreC),
            22 => Ok(Self::AddI),
            23 => Ok(Self::Add),
            24 => Ok(Self::Sub),
            25 => Ok(Self::SltsI),
            26 => Ok(Self::SltuI),
            27 => Ok(Self::Slts),
            28 => Ok(Self::Sltu),
            29 => Ok(Self::XorI),
            30 => Ok(Self::Xor),
            31 => Ok(Self::OrI),
            32 => Ok(Self::Or),
            33 => Ok(Self::AndI),
            34 => Ok(Self::And),
            35 => Ok(Self::SllI),
            36 => Ok(Self::Sll),
            37 => Ok(Self::SrlI),
            38 => Ok(Self::Srl),
            39 => Ok(Self::SraI),
            40 => Ok(Self::Sra),
            41 => Ok(Self::Jal),
            42 => Ok(Self::Jalr),
            43 => Ok(Self::Beq),
            44 => Ok(Self::Bne),
            45 => Ok(Self::Blts),
            46 => Ok(Self::Bges),
            47 => Ok(Self::Bltu),
            48 => Ok(Self::Bgeu),
            49 => Ok(Self::Syscall),
            _ => Err(Exception::InvalidOpKind { byte }),
        }
    }

    pub const fn operand_count(self) -> u8 {
        match self {
            Self::Nop => 0,
            Self::CGetAddr => 2,
            Self::CSetAddr => 2,
            Self::CGetBound => 3,
            Self::CSetBound => 3,
            Self::CGetPerm => 2,
            Self::CSetPerm => 2,
            Self::CGetValid => 2,
            Self::Cpy => 2,
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
}

impl Ty for OpKind {
    const LAYOUT: Layout = Layout {
        size: 1,
        align: Align::new(1).unwrap(),
    };

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        Self::from_byte(u8::read(src, addr, valid)?)
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        self.to_byte().write(dst, addr, valid)
    }
}

impl fmt::Display for OpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Nop => "nop",
            Self::CGetAddr => "cgetaddr",
            Self::CSetAddr => "csetaddr",
            Self::CGetBound => "cgetbound",
            Self::CSetBound => "csetbound",
            Self::CGetPerm => "cgetperm",
            Self::CSetPerm => "csetperm",
            Self::CGetValid => "cgetvalid",
            Self::Cpy => "cpy",
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
    const FIELDS: &'static [Layout] = &[
        OpKind::LAYOUT,
        TaggedCapability::LAYOUT,
        TaggedCapability::LAYOUT,
        TaggedCapability::LAYOUT,
    ];
}

impl Ty for Op {
    const LAYOUT: Layout = abi::layout(Self::FIELDS);

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        let mut fields = StructRef::new(src, addr, valid, Self::FIELDS);
        Ok(Self {
            kind: fields.read_next::<OpKind>()?,
            op1: fields.read_next::<TaggedCapability>()?,
            op2: fields.read_next::<TaggedCapability>()?,
            op3: fields.read_next::<TaggedCapability>()?,
        })
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        let mut fields = StructMut::new(dst, addr, valid, Self::FIELDS);
        fields.write_next(self.kind)?;
        fields.write_next(self.op1)?;
        fields.write_next(self.op2)?;
        fields.write_next(self.op3)?;
        Ok(())
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        let op_count = self.kind.operand_count();
        if op_count > 0 {
            write!(f, " ")?;
        }
        for (i, op) in [self.op1, self.op2, self.op3]
            .into_iter()
            .take(op_count.into())
            .enumerate()
        {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{op:?}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_struct("Op");
        dbg.field("kind", &self.kind);
        for (i, op) in [self.op1, self.op2, self.op3]
            .into_iter()
            .take(self.kind.operand_count().into())
            .enumerate()
        {
            dbg.field(&format!("op{i}", i = i + 1), &op);
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
