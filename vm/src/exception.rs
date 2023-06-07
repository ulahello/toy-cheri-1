use core::fmt;
use std::io;

use crate::access::{MemAccess, RegAccess};

#[derive(Clone, Copy, Debug)]
pub enum Exception {
    InvalidOpKind { byte: u8 },

    InvalidSyscall { byte: u8 },

    InvalidMemAccess { access: MemAccess },

    InvalidRegAccess { access: RegAccess },

    ProcessExit,
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidOpKind { byte } => {
                write!(f, "invalid operation kind 0x{byte:x}")?;
            }

            Self::InvalidSyscall { byte } => {
                write!(f, "invalid system call 0x{byte:x}")?;
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
                if access.is_reg_valid() {
                    write!(f, "no such register")?;
                } else {
                    unreachable!("valid register access is not exception");
                }
            }

            Self::ProcessExit => write!(f, "process exited")?,
        }
        Ok(())
    }
}

impl std::error::Error for Exception {}

#[derive(Debug)]
pub enum VmException {
    Userspace(Exception),
    Io(io::Error),
}

impl fmt::Display for VmException {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Userspace(err) => write!(f, "userspace exception: {err}")?,
            Self::Io(err) => write!(f, "input/output: {err}")?,
        }
        Ok(())
    }
}

impl From<io::Error> for VmException {
    fn from(io: io::Error) -> Self {
        Self::Io(io)
    }
}

impl From<Exception> for VmException {
    fn from(user: Exception) -> Self {
        Self::Userspace(user)
    }
}

impl std::error::Error for VmException {}
