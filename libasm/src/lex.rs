use fruticose_vm::int::{UAddr, UGran, UADDR_SIZE, UGRAN_SIZE};
use fruticose_vm::op::OpKind;
use fruticose_vm::registers::Register;
use fruticose_vm::syscall::SyscallKind;
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

use core::iter::Peekable;
use core::num::{IntErrorKind, ParseIntError};

use crate::parse1::OperandType;
use crate::Span;

pub const COMMENT: &str = ";";

// TODO: warn about suspicious unicode characters

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token<'s> {
    pub(crate) typ: TokenTyp,
    pub(crate) span: Span<'s>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenTyp {
    // require adjacent: whitespace or start/end of file
    Op(OpKind),
    Register(Register),
    Syscall(SyscallKind),
    UnsignedInt(UGran), // TODO: support signed ints. also we should have a clear way to notate type of literal (eg, <number>_s for signed and <number>_u for unsigned)
    Identifier,

    // if seen, immediately yield
    Comma,
    Colon,
    Newline,

    Eof,
}

impl TokenTyp {
    pub const fn operand_type(self) -> Option<OperandType> {
        match self {
            Self::Register(_) => Some(OperandType::Register),
            Self::Syscall(_) | Self::UnsignedInt(_) => Some(OperandType::Immediate),
            Self::Identifier => Some(OperandType::Label),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexErr<'s> {
    pub(crate) typ: LexErrTyp,
    pub(crate) span: Span<'s>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LexErrTyp {
    UnknownIdent,
    InvalidUnsignedInt(ParseIntError),
}

pub struct Lexer<'s> {
    src: &'s str,
    graphs: Peekable<GraphemeIndices<'s>>,
    line: usize,
    line_start: usize,
    eof: bool,
}

impl<'s> Lexer<'s> {
    pub fn new(src: &'s str) -> Self {
        Self {
            src,
            graphs: UnicodeSegmentation::grapheme_indices(src, true).peekable(),
            line: 0,
            line_start: 0,
            eof: false,
        }
    }
}

impl<'s> Lexer<'s> {
    #[must_use]
    fn check_no_ctx(chr: &'s str) -> Option<TokenTyp> {
        match chr {
            "," => Some(TokenTyp::Comma),
            ":" => Some(TokenTyp::Colon),
            "\n" => Some(TokenTyp::Newline),
            _ => None,
        }
    }

    fn check_ctx(span: &'s str) -> Result<TokenTyp, LexErrTyp> {
        let typ = if let Some(op) = OpKind::from_str(span) {
            TokenTyp::Op(op)
        } else if let Some(reg) = Register::from_str(span) {
            TokenTyp::Register(reg)
        } else {
            match span {
                // syscalls
                "SYS_EXIT" => TokenTyp::Syscall(SyscallKind::Exit),
                "SYS_ALLOC_INIT" => TokenTyp::Syscall(SyscallKind::AllocInit),
                "SYS_ALLOC_DEINIT" => TokenTyp::Syscall(SyscallKind::AllocDeInit),
                "SYS_ALLOC_ALLOC" => TokenTyp::Syscall(SyscallKind::AllocAlloc),
                "SYS_ALLOC_FREE" => TokenTyp::Syscall(SyscallKind::AllocFree),
                "SYS_ALLOC_FREE_ALL" => TokenTyp::Syscall(SyscallKind::AllocFreeAll),
                "SYS_ALLOC_STAT" => TokenTyp::Syscall(SyscallKind::AllocStat),

                // helpful constants
                "UGRAN_SIZE" => TokenTyp::UnsignedInt(UGRAN_SIZE.into()),
                "UGRAN_BITS" => TokenTyp::UnsignedInt(UGran::BITS.into()),
                "UADDR_SIZE" => TokenTyp::UnsignedInt(UADDR_SIZE.into()),
                "UADDR_BITS" => TokenTyp::UnsignedInt(UAddr::BITS.into()),

                _ => match span.parse::<UGran>() {
                    Ok(int) => TokenTyp::UnsignedInt(int),
                    Err(err) => {
                        if *err.kind() == IntErrorKind::PosOverflow {
                            return Err(LexErrTyp::InvalidUnsignedInt(err));
                        }
                        TokenTyp::Identifier
                    }
                },
            }
        };
        Ok(typ)
    }

    fn next_inner(&mut self) -> Option<<Self as Iterator>::Item> {
        /* skip into non-whitespace, yielding context-independent token if we
         * find one */
        let mut ctx = loop {
            let (mut idx, mut chr) = self.graphs.next()?;
            if chr == COMMENT {
                // pretend we didn't see that
                loop {
                    let (try_idx, try_chr) = self.graphs.next()?;
                    if try_chr == "\n" {
                        idx = try_idx;
                        chr = try_chr;
                        break;
                    }
                }
            }

            let ctx = Span {
                line: self.line,
                col_idx: idx - self.line_start,
                len: chr.len(),
                line_start: self.line_start,
                src: self.src,
            };

            if let Some(typ) = Self::check_no_ctx(chr) {
                if typ == TokenTyp::Newline {
                    self.line += 1;
                    self.line_start = idx + 1;
                }
                return Some(Ok(Token { typ, span: ctx }));
            }

            if chr.chars().all(char::is_whitespace) {
                continue;
            }

            break ctx;
        };

        /* fill context-dependent span for as long as we can */
        while let Some((_, chr)) = self.graphs.peek().copied() {
            if Self::check_no_ctx(chr).is_some() {
                /* can't keep filling ctx. checking context-independent
                 * tokens incidentally prevents these spans from
                 * breaking into subsequent lines. */
                break;
            }
            if chr.chars().all(char::is_whitespace) {
                // ctx delimited by whitespace, can't keep filling.
                break;
            }
            if chr == COMMENT {
                // start of comment, can't keep filling.
                break;
            }

            ctx.len += chr.len();
            self.graphs.next();
            continue;
        }

        /* check context-dependent span! */
        match Self::check_ctx(ctx.get()) {
            Ok(typ) => Some(Ok(Token { typ, span: ctx })),
            Err(err) => Some(Err(LexErr {
                typ: err,
                span: ctx,
            })),
        }
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Result<Token<'s>, LexErr<'s>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }
        if let Some(next) = self.next_inner() {
            Some(next)
        } else {
            // eof encountered!
            self.eof = true;
            let abs_idx = self.src.len();
            Some(Ok(Token {
                typ: TokenTyp::Eof,
                span: Span {
                    line: self.line,
                    col_idx: abs_idx - self.line_start,
                    len: 0,
                    line_start: self.line_start,
                    src: self.src,
                },
            }))
        }
    }
}
