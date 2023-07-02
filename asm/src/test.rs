use crate::lex::{ByteSpan, Lexer, Token, TokenTyp};
use fruticose_vm::op::OpKind;
use fruticose_vm::registers::Register;
use fruticose_vm::syscall::SyscallKind;

#[test]
fn exit() {
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
    assert_eq!(lexer.next(), None);
}
