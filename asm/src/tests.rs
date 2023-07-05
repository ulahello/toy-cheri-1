use fruticose_vm::capability::TaggedCapability;
use fruticose_vm::op::{Op, OpKind};
use fruticose_vm::registers::Register;
use fruticose_vm::syscall::SyscallKind;

use crate::lex::{Lexer, Token, TokenTyp};
use crate::parse::Parser;
use crate::Span;

const EXIT: &str = include_str!("../examples/exit.asm");
const CRASH_1: &str = include_str!("../examples/crash-1.asm");
const INT_LITERAL: &str = include_str!("../examples/int-literal.asm");

#[test]
fn exit_lex() {
    let src = EXIT;
    let mut lexer = Lexer::new(src);
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::Nop),
            span: Span {
                line: 0,
                col_idx: 0,
                len: 3,
                line_start: 0,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 0,
                col_idx: 22,
                len: 1,
                line_start: 0,
                src,
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::LoadI),
            span: Span {
                line: 1,
                col_idx: 0,
                len: 5,
                line_start: 23,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::A0),
            span: Span {
                line: 1,
                col_idx: 6,
                len: 2,
                line_start: 23,
                src,
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: Span {
                line: 1,
                col_idx: 8,
                len: 1,
                line_start: 23,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Syscall(SyscallKind::Exit),
            span: Span {
                line: 1,
                col_idx: 10,
                len: 8,
                line_start: 23,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 1,
                col_idx: 18,
                len: 1,
                line_start: 23,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::Syscall),
            span: Span {
                line: 2,
                col_idx: 0,
                len: 7,
                line_start: 42,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 2,
                col_idx: 7,
                len: 1,
                line_start: 42,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Eof,
            span: Span {
                line: 3,
                col_idx: 0,
                len: 0,
                line_start: 50,
                src,
            },
        }))
    );
    assert_eq!(lexer.next(), None);
}

#[test]
fn exit_parse() {
    let src = EXIT;
    let mut parser = Parser::new(src);
    assert_eq!(parser.next(), Some(Ok(Op::nop())));
    assert_eq!(
        parser.next(),
        Some(Ok(Op::loadi(
            Register::A0 as _,
            TaggedCapability::from_ugran(SyscallKind::Exit as _),
        )))
    );
    assert_eq!(parser.next(), Some(Ok(Op::syscall())));
    assert_eq!(parser.next(), None);
}

#[test]
fn crash_1_lex() {
    let src = CRASH_1;
    let mut lexer = Lexer::new(src);
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 0,
                col_idx: 95,
                len: 1,
                line_start: 0,
                src,
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::LoadI),
            span: Span {
                line: 1,
                col_idx: 0,
                len: 5,
                line_start: 96,
                src,
            }
        }))
    );
}

#[test]
fn int_literal_lex() {
    let src = INT_LITERAL;
    let mut lexer = Lexer::new(src);
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::LoadI),
            span: Span {
                line: 0,
                col_idx: 0,
                len: 5,
                line_start: 0,
                src,
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::T1),
            span: Span {
                line: 0,
                col_idx: 6,
                len: 2,
                line_start: 0,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: Span {
                line: 0,
                col_idx: 8,
                len: 1,
                line_start: 0,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::UnsignedInt(23),
            span: Span {
                line: 0,
                col_idx: 10,
                len: 2,
                line_start: 0,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 0,
                col_idx: 12,
                len: 1,
                line_start: 0,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::LoadI),
            span: Span {
                line: 1,
                col_idx: 0,
                len: 5,
                line_start: 13,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::T2),
            span: Span {
                line: 1,
                col_idx: 6,
                len: 2,
                line_start: 13,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: Span {
                line: 1,
                col_idx: 8,
                len: 1,
                line_start: 13,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::UnsignedInt(47),
            span: Span {
                line: 1,
                col_idx: 10,
                len: 2,
                line_start: 13,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 1,
                col_idx: 12,
                len: 1,
                line_start: 13,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::Add),
            span: Span {
                line: 2,
                col_idx: 0,
                len: 3,
                line_start: 26,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::T0),
            span: Span {
                line: 2,
                col_idx: 4,
                len: 2,
                line_start: 26,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: Span {
                line: 2,
                col_idx: 6,
                len: 1,
                line_start: 26,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::T1),
            span: Span {
                line: 2,
                col_idx: 8,
                len: 2,
                line_start: 26,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: Span {
                line: 2,
                col_idx: 10,
                len: 1,
                line_start: 26,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::T2),
            span: Span {
                line: 2,
                col_idx: 12,
                len: 2,
                line_start: 26,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 2,
                col_idx: 14,
                len: 1,
                line_start: 26,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 3,
                col_idx: 23,
                len: 1,
                line_start: 41,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 4,
                col_idx: 0,
                len: 1,
                line_start: 65,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::LoadI),
            span: Span {
                line: 5,
                col_idx: 0,
                len: 5,
                line_start: 66,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::A0),
            span: Span {
                line: 5,
                col_idx: 6,
                len: 2,
                line_start: 66,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: Span {
                line: 5,
                col_idx: 8,
                len: 1,
                line_start: 66,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Syscall(SyscallKind::Exit),
            span: Span {
                line: 5,
                col_idx: 10,
                len: 8,
                line_start: 66,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 5,
                col_idx: 18,
                len: 1,
                line_start: 66,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::Syscall),
            span: Span {
                line: 6,
                col_idx: 0,
                len: 7,
                line_start: 85,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 6,
                col_idx: 7,
                len: 1,
                line_start: 85,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Eof,
            span: Span {
                line: 7,
                col_idx: 0,
                len: 0,
                line_start: 93,
                src
            }
        }))
    );
    assert_eq!(lexer.next(), None);
}
