use fruticose_vm::op::OpKind;
use fruticose_vm::registers::Register;
use fruticose_vm::syscall::SyscallKind;
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

use core::iter::Peekable;

// TODO: copypasted from `sw`. does this merit a lib?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ByteSpan<'s> {
    start: usize,
    len: usize,
    src: &'s str,
}

impl<'s> ByteSpan<'s> {
    #[must_use]
    pub const fn new(start: usize, len: usize, s: &'s str) -> Self {
        Self { start, len, src: s }
    }

    pub fn shift_start_left(&mut self, bytes: usize) {
        self.start -= bytes;
        self.len += bytes;
    }

    pub fn shift_start_right(&mut self, bytes: usize) {
        self.start += bytes;
        self.len -= bytes;
    }

    pub fn get(&self) -> &'s str {
        &self.src[self.start..self.start + self.len]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token<'s> {
    pub(crate) typ: TokenTyp,
    pub(crate) span: ByteSpan<'s>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenTyp {
    Op(OpKind),
    Register(Register),
    Syscall(SyscallKind),
    Ident,
    Comma,
    Newline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LexErr<'s> {
    typ: LexErrTyp,
    span: ByteSpan<'s>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexErrTyp {}

pub struct Lexer<'s> {
    graphs: Peekable<GraphemeIndices<'s>>,
    src: &'s str,
}

enum Next<'s> {
    Tok(Token<'s>),
    Char((usize, &'s str)),
    Eof,
}

impl<'s> Lexer<'s> {
    pub fn new(src: &'s str) -> Self {
        Self {
            graphs: UnicodeSegmentation::grapheme_indices(src, true).peekable(),
            src,
        }
    }

    fn consume_grapheme(&mut self) -> Next<'s> {
        let next = self.peek_grapheme();
        self.graphs.next();
        next
    }

    fn peek_grapheme(&mut self) -> Next<'s> {
        if let Some((idx, chr)) = self.graphs.peek().copied() {
            // either tok or char
            let typ = match chr {
                "\n" => Some(TokenTyp::Newline),
                "," => Some(TokenTyp::Comma),
                _ => None,
            };
            if let Some(typ) = typ {
                Next::Tok(Token {
                    typ,
                    span: ByteSpan::new(idx, chr.len(), self.src),
                })
            } else {
                Next::Char((idx, chr))
            }
        } else {
            Next::Eof
        }
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = Result<Token<'s>, LexErr<'s>>;

    fn next(&mut self) -> Option<Self::Item> {
        // skip into non-whitespace
        let (idx, chr) = loop {
            match self.consume_grapheme() {
                Next::Tok(tok) => return Some(Ok(tok)),
                Next::Char((idx, chr)) => {
                    // skip whitespace
                    if chr.chars().all(char::is_whitespace) {
                        continue;
                    } else {
                        break (idx, chr);
                    }
                }
                Next::Eof => return None,
            }
        };

        let mut span = ByteSpan::new(idx, chr.len(), self.src);

        // skip until end of possible ident
        loop {
            match self.peek_grapheme() {
                Next::Tok(_) | Next::Eof => {
                    // end of possible ident
                    break;
                }
                Next::Char((_, chr)) => {
                    if chr.chars().all(char::is_whitespace) {
                        // end of possible ident
                        break;
                    } else {
                        // still more characters!
                        span.len += chr.len();
                        self.graphs.next();
                        continue;
                    }
                }
            }
        }

        let typ = match span.get() {
            "nop" => TokenTyp::Op(OpKind::Nop),
            "loadi" => TokenTyp::Op(OpKind::LoadI),
            "syscall" => TokenTyp::Op(OpKind::Syscall),

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

            "SYS_EXIT" => TokenTyp::Syscall(SyscallKind::Exit),

            _ => TokenTyp::Ident,
        };
        Some(Ok(Token { typ, span }))
    }
}
