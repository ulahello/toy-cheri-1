use fruticose_vm::capability::TaggedCapability;
use fruticose_vm::op::{Op, OpKind};
use fruticose_vm::registers::Register;
use fruticose_vm::syscall::SyscallKind;

use crate::lex::{Lexer, Token, TokenTyp};
use crate::parse2::Parser2;
use crate::Span;

const EXIT: &str = include_str!("../examples/exit.asm");
const ADD: &str = include_str!("../examples/add.asm");

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
    let mut parser = Parser2::new(src);
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
fn add_lex() {
    let src = ADD;
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
                col_idx: 16,
                len: 1,
                line_start: 41,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::AddI),
            span: Span {
                line: 4,
                col_idx: 0,
                len: 4,
                line_start: 58,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::T0),
            span: Span {
                line: 4,
                col_idx: 5,
                len: 2,
                line_start: 58,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: Span {
                line: 4,
                col_idx: 7,
                len: 1,
                line_start: 58,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::T0),
            span: Span {
                line: 4,
                col_idx: 9,
                len: 2,
                line_start: 58,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: Span {
                line: 4,
                col_idx: 11,
                len: 1,
                line_start: 58,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::UnsignedInt(1),
            span: Span {
                line: 4,
                col_idx: 13,
                len: 1,
                line_start: 58,
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
                col_idx: 14,
                len: 1,
                line_start: 58,
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
                col_idx: 16,
                len: 1,
                line_start: 73,
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
                col_idx: 0,
                len: 1,
                line_start: 90,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::LoadI),
            span: Span {
                line: 7,
                col_idx: 0,
                len: 5,
                line_start: 91,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Register(Register::A0),
            span: Span {
                line: 7,
                col_idx: 6,
                len: 2,
                line_start: 91,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Comma,
            span: Span {
                line: 7,
                col_idx: 8,
                len: 1,
                line_start: 91,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Syscall(SyscallKind::Exit),
            span: Span {
                line: 7,
                col_idx: 10,
                len: 8,
                line_start: 91,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 7,
                col_idx: 18,
                len: 1,
                line_start: 91,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Op(OpKind::Syscall),
            span: Span {
                line: 8,
                col_idx: 0,
                len: 7,
                line_start: 110,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Newline,
            span: Span {
                line: 8,
                col_idx: 7,
                len: 1,
                line_start: 110,
                src
            }
        }))
    );
    assert_eq!(
        lexer.next(),
        Some(Ok(Token {
            typ: TokenTyp::Eof,
            span: Span {
                line: 9,
                col_idx: 0,
                len: 0,
                line_start: 118,
                src
            }
        }))
    );
    assert_eq!(lexer.next(), None);
}

mod crash {
    use fruticose_vm::op::OpKind;

    use crate::lex::{Lexer, Token, TokenTyp};
    use crate::parse1::{ParseErr, ParseErrTyp};
    use crate::parse2::Parser2;
    use crate::Span;

    const CRASH_3: &str = include_str!("../examples/crash-3.asm");

    #[test]
    fn parser_eof_unreachable() {
        let src = CRASH_3;

        {
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
                        src
                    }
                }))
            );
            assert_eq!(
                lexer.next(),
                Some(Ok(Token {
                    typ: TokenTyp::Eof,
                    span: Span {
                        line: 0,
                        col_idx: 5,
                        len: 0,
                        line_start: 0,
                        src
                    }
                }))
            );
            assert_eq!(lexer.next(), None);
        }

        {
            let mut parser = Parser2::new(src);
            assert_eq!(
                parser.next(),
                Some(Err(ParseErr {
                    typ: ParseErrTyp::InvalidOperand,
                    span: Span {
                        line: 0,
                        col_idx: 5,
                        len: 0,
                        line_start: 0,
                        src,
                    }
                }))
            );
        }
    }
}
