use core::fmt;

use crate::access::{MemAccess, RegAccess};
use crate::alloc::{AllocErr, AllocErrKind};
use crate::int::UAddr;

#[derive(Clone, Copy, Debug)]
pub enum Exception {
    InvalidOpKind { byte: u8 },

    InvalidSyscall { byte: u8 },

    InvalidAllocStrategy { byte: u8 },

    InvalidAllocInitFlags { flags: u8 },

    InvalidAlign { align: UAddr },

    InvalidMemAccess { access: MemAccess },

    InvalidRegAccess { access: RegAccess },

    AllocErr { err: AllocErr },

    ProcessExit,
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOpKind { byte } => {
                write!(f, "invalid operation kind 0x{byte:x}")?;
            }

            Self::InvalidSyscall { byte } => {
                write!(f, "invalid system call 0x{byte:x}")?;
            }

            Self::InvalidAllocStrategy { byte } => {
                write!(f, "invalid allocation strategy {byte}")?;
            }

            Self::InvalidAllocInitFlags { flags } => {
                write!(f, "invalid allocator init flags 0b{flags:0b}")?;
            }

            Self::InvalidAlign { align } => {
                write!(f, "invalid alignment {align}: must be a power of two")?;
            }

            Self::InvalidMemAccess { access } => {
                write!(
                    f,
                    "invalid memory access ({kind} of {perms}) of ",
                    kind = access.kind,
                    perms = access.tcap.perms()
                )?;
                if let Some(len) = access.len {
                    write!(
                        f,
                        "{len} byte{bytes_plural}",
                        bytes_plural = if len == 1 { "" } else { "s" }
                    )?;
                } else {
                    write!(f, "overflowing number of bytes")?;
                }
                write!(
                    f,
                    " with alignment of {align} at {addr}: ",
                    align = access.align,
                    addr = access.tcap.addr()
                )?;

                let start = access.tcap.start();
                let endb = access.tcap.endb();
                if !access.tcap.is_valid() {
                    write!(f, "data used where capability required")?;
                    return Ok(());
                }
                if !access.is_bounded() {
                    write!(f, "access uncontained by bounds {start}..{endb}")?;
                    return Ok(());
                }
                if !access.perms_grant() {
                    write!(
                        f,
                        "no permission {perm} granted for bounds {start}..{endb}",
                        perm = access.kind
                    )?;
                    return Ok(());
                }
                if !access.is_aligned() {
                    write!(f, "access unaligned")?;
                    return Ok(());
                }
                assert!(
                    access.tcap.check_given_access(*access).is_ok(),
                    "dev forgot to handle case why this access is invalid"
                );
                unreachable!("valid memory access is not exception");
            }

            Self::InvalidRegAccess { access } => {
                write!(f, "invalid access of register {reg}: ", reg = access.reg)?;
                if !access.is_reg_valid() {
                    write!(f, "no such register")?;
                    return Ok(());
                }
                if !access.is_len_valid() {
                    write!(
                        f,
                        "access of {} byte{s} exceeds register size",
                        access.len,
                        s = if access.len == 1 { "" } else { "s" },
                    )?;
                    return Ok(());
                }
                assert!(
                    access.check().is_ok(),
                    "dev forgot to handle case why this access is invalid"
                );
                unreachable!("valid register access is not exception");
            }

            Self::AllocErr { err } => {
                write!(
                    f,
                    "allocator reported error: stats = {stats:?}, requested = {requested:?}: ",
                    stats = err.stats,
                    requested = err.requested
                )?;
                match err.kind {
                    AllocErrKind::NotEnoughMem => write!(f, "not enough memory")?,
                    AllocErrKind::Oom => write!(f, "out of memory")?,
                }
            }

            Self::ProcessExit => write!(f, "process exited")?,
        }
        Ok(())
    }
}

impl std::error::Error for Exception {}
