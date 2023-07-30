use core::fmt;

use crate::lex::TokenTyp;
use crate::parse1::{OperandType, TokenClass};

impl fmt::Display for TokenTyp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = self.classify();
        match self {
            Self::Op(op) => write!(f, "{class} {op}", class = class.unwrap()),
            Self::Register(reg) => write!(f, "{class} {reg}", class = class.unwrap()),
            Self::Syscall(syscall) => write!(f, "{class} {syscall}", class = class.unwrap()),
            Self::UnsignedInt(_) => write!(f, "{class} number", class = class.unwrap()),
            Self::Identifier => write!(f, "identifier"),
            Self::Comma => write!(f, "comma"),
            Self::Colon => write!(f, "colon"),
            Self::Newline => write!(f, "newline"),
            Self::Eof => write!(f, "end of file"),
        }
    }
}

impl fmt::Display for TokenClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Op => "operation",
            Self::Register => "register",
            Self::Syscall => "syscall",
            Self::Literal => "literal",
            Self::Identifier => "identifier",
        };
        f.write_str(s)
    }
}

impl fmt::Display for OperandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Register => "register",
            Self::Immediate => "immediate",
            Self::Label => "label",
            Self::Unused => unreachable!(),
        };
        f.write_str(s)
    }
}
