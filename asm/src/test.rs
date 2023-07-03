use fruticose_vm::capability::TaggedCapability;
use fruticose_vm::op::{Op, OpKind};
use fruticose_vm::registers::Register;
use fruticose_vm::syscall::SyscallKind;

use crate::lex::{ByteSpan, Lexer, Token, TokenTyp};
use crate::parse::Parser;

#[test]
fn exit_lex() {
    let src = include_str!("../examples/exit.asm");
    let mut lexer = Lexer::new(src);
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::Nop),
            span: ByteSpan::new(0, 3, src)
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: ByteSpan::new(22, 1, src)
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::LoadI),
            span: ByteSpan::new(23, 5, src)
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::A0),
            span: ByteSpan::new(29, 2, src)
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: ByteSpan::new(31, 1, src)
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Syscall(SyscallKind::Exit),
            span: ByteSpan::new(33, 8, src)
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: ByteSpan::new(41, 1, src)
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::Syscall),
            span: ByteSpan::new(42, 7, src)
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: ByteSpan::new(49, 1, src)
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Eof,
            span: ByteSpan::new(50, 0, src)
        }))
    );
    assert_eq!(lexer.next(), None);
}

#[test]
fn exit_parse() {
    let src = include_str!("../examples/exit.asm");
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
