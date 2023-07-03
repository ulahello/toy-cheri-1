use fruticose_vm::capability::TaggedCapability;
use fruticose_vm::op::{Op, OpKind};
use fruticose_vm::registers::Register;
use fruticose_vm::syscall::SyscallKind;

use crate::lex::{Lexer, Token, TokenTyp};
use crate::parse::Parser;
use crate::Span;

const EXIT: &str = include_str!("../examples/exit.asm");

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
