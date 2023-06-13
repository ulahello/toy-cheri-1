use crate::exception::Exception;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SyscallKind {
    /// Exit the process.
    Exit = 0,
}

impl SyscallKind {
    pub const fn to_byte(self) -> u8 {
        // NOTE: guarenteed and depended to be the discriminant
        self as u8
    }

    pub const fn from_byte(byte: u8) -> Result<Self, Exception> {
        match byte {
            0 => Ok(Self::Exit),
            _ => Err(Exception::InvalidSyscall { byte }),
        }
    }
}
