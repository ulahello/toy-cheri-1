use core::fmt;

use crate::exception::Exception;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SyscallKind {
    /// Exit the process.
    Exit = 0,

    /// Initialize an allocator by giving it ownership over the region of memory
    /// represented by the capability at register `a5`, the [allocation
    /// strategy](crate::alloc::Strategy) at register `a3`, and the
    /// [flags](crate::alloc::InitFlags) at register `a4`. On success, the
    /// allocator capability is written to register `a0`.
    AllocInit,

    // TODO: not implemented, docs subject to change
    /// De-initialize the allocator at register `a3`. This will invalidate all
    /// allocations currently yielded by the allocator, and the allocator
    /// itself. On success, the region previously owned by the allocator
    /// (represented by a capability) is written to register `a0`.
    AllocDeInit,

    /// Request an allocation from the allocator at register `a3` with the
    /// [`Layout`](crate::abi::Layout) at register `a4`. On success, a
    /// capability to the allocation is written to register `a0`.
    AllocAlloc,

    // TODO: not implemented, docs subject to change
    /// Free a previously requested allocation from the allocator at register
    /// `a3` with the allocation capability at register `a4`.
    AllocFree,

    /// Free all allocations yielded by the allocator at register `a3`.
    AllocFreeAll,

    /// Request [`Stats`](crate::alloc::Stats) on the allocator at register
    /// `a3`.
    AllocStat,
}

impl SyscallKind {
    pub const fn to_byte(self) -> u8 {
        // NOTE: guarenteed and depended to be the discriminant
        self as u8
    }

    pub const fn from_byte(byte: u8) -> Result<Self, Exception> {
        match byte {
            0 => Ok(Self::Exit),
            1 => Ok(Self::AllocInit),
            2 => Ok(Self::AllocDeInit),
            3 => Ok(Self::AllocAlloc),
            4 => Ok(Self::AllocFree),
            5 => Ok(Self::AllocFreeAll),
            6 => Ok(Self::AllocStat),
            _ => Err(Exception::InvalidSyscall { byte }),
        }
    }
}

impl fmt::Display for SyscallKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Exit => "Exit",
            Self::AllocInit => "AllocInit",
            Self::AllocDeInit => "AllocDeInit",
            Self::AllocAlloc => "AllocAlloc",
            Self::AllocFree => "AllocFree",
            Self::AllocFreeAll => "AllocFreeAll",
            Self::AllocStat => "AllocStat",
        };
        f.write_str(s)
    }
}
