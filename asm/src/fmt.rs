use core::fmt;

use crate::lex::TokenTyp;
use crate::parse::TokenClass;

impl fmt::Display for TokenTyp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = self.classify();
        match self {
            Self::Op(op) => write!(f, "{class} '{op}'", class = class.unwrap()),
            Self::Register(reg) => write!(f, "{class} '{reg}'", class = class.unwrap()),
            Self::Syscall(syscall) => write!(f, "{class} '{syscall}'", class = class.unwrap()),
            Self::Comma => write!(f, "comma"),
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
        };
        f.write_str(s)
    }
}
