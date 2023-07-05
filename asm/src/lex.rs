use fruticose_vm::int::UGran;
use fruticose_vm::op::OpKind;
use fruticose_vm::registers::Register;
use fruticose_vm::syscall::SyscallKind;
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

use core::iter::Peekable;
use core::num::{IntErrorKind, ParseIntError};

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

    // if seen, immediately yield
    Comma,
    Newline,

    Eof,
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
            "\n" => Some(TokenTyp::Newline),
            "," => Some(TokenTyp::Comma),
            _ => None,
        }
    }

    #[must_use]
    fn check_ctx(span: &'s str) -> Result<TokenTyp, LexErrTyp> {
        let typ = match span {
            // operations
            "nop" => TokenTyp::Op(OpKind::Nop),
            "loadi" => TokenTyp::Op(OpKind::LoadI),
            "syscall" => TokenTyp::Op(OpKind::Syscall),

            // registers
            "zero" => TokenTyp::Register(Register::Zero),
            "pc" => TokenTyp::Register(Register::Pc),
            "ra" => TokenTyp::Register(Register::Ra),
            "sp" => TokenTyp::Register(Register::Sp),
            "t0" => TokenTyp::Register(Register::T0),
            "t1" => TokenTyp::Register(Register::T1),
            "t2" => TokenTyp::Register(Register::T2),
            "t3" => TokenTyp::Register(Register::T3),
            "t4" => TokenTyp::Register(Register::T4),
            "t5" => TokenTyp::Register(Register::T5),
            "t6" => TokenTyp::Register(Register::T6),
            "a0" => TokenTyp::Register(Register::A0),
            "a1" => TokenTyp::Register(Register::A1),
            "a2" => TokenTyp::Register(Register::A2),
            "a3" => TokenTyp::Register(Register::A3),
            "a4" => TokenTyp::Register(Register::A4),
            "a5" => TokenTyp::Register(Register::A5),
            "a6" => TokenTyp::Register(Register::A6),
            "a7" => TokenTyp::Register(Register::A7),
            "s0" => TokenTyp::Register(Register::S0),
            "s1" => TokenTyp::Register(Register::S1),
            "s2" => TokenTyp::Register(Register::S2),
            "s3" => TokenTyp::Register(Register::S3),
            "s4" => TokenTyp::Register(Register::S4),
            "s5" => TokenTyp::Register(Register::S5),
            "s6" => TokenTyp::Register(Register::S6),
            "s7" => TokenTyp::Register(Register::S7),
            "s8" => TokenTyp::Register(Register::S8),
            "s9" => TokenTyp::Register(Register::S9),
            "s10" => TokenTyp::Register(Register::S10),
            "s11" => TokenTyp::Register(Register::S11),
            "z0" => TokenTyp::Register(Register::Z0),

            // syscalls
            "SYS_EXIT" => TokenTyp::Syscall(SyscallKind::Exit),

            _ => match span.parse::<UGran>() {
                Ok(int) => TokenTyp::UnsignedInt(int),
                Err(err) => {
                    if *err.kind() == IntErrorKind::PosOverflow {
                        return Err(LexErrTyp::InvalidUnsignedInt(err));
                    }
                    return Err(LexErrTyp::UnknownIdent);
                }
            },
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
