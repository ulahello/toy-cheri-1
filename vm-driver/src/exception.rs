use fruticose_asm::parse::ParseErr;
use fruticose_vm::exception::Exception;

use core::fmt;
use std::io;

#[derive(Debug)]
pub enum VmException {
    Userspace(Exception),
    Io(io::Error),
    AssembleInit,
}

impl fmt::Display for VmException {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Userspace(err) => write!(f, "userspace exception: {err}")?,
            Self::Io(err) => write!(f, "input/output: {err}")?,
            Self::AssembleInit => write!(f, "failed to assemble init program")?,
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

impl<'s> From<ParseErr<'s>> for VmException {
    fn from(_err: ParseErr<'s>) -> Self {
        Self::AssembleInit
    }
}

impl std::error::Error for VmException {}
