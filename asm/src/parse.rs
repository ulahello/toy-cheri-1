use fruticose_vm::capability::TaggedCapability;
use fruticose_vm::op::{Op, OpKind, OperandType};

use crate::lex::{LexErrTyp, Lexer, Token, TokenTyp};
use crate::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseErr<'s> {
    pub typ: ParseErrTyp,
    pub span: Span<'s>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenClass {
    Op,
    Register,
    Syscall,
    Literal,
}

impl TokenTyp {
    pub const fn classify(self) -> Option<TokenClass> {
        match self {
            Self::Op(_) => Some(TokenClass::Op),
            Self::Register(_) => Some(TokenClass::Register),
            Self::Syscall(_) => Some(TokenClass::Syscall),
            Self::UnsignedInt(_) => Some(TokenClass::Literal),
            Self::Comma | Self::Newline | Self::Eof => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrTyp {
    Lex(LexErrTyp),
    ExpectedTyp {
        expected: TokenTyp,
        found: TokenTyp,
    },
    ExpectedClass {
        expected: TokenClass,
        found: TokenTyp,
    },
    InvalidOperand,
    OperandTypeMismatch {
        expected: OperandType,
        found: OperandType,
    },
}

pub struct Parser<'s> {
    lexer: Lexer<'s>,
}

impl<'s> Parser<'s> {
    pub fn new(src: &'s str) -> Self {
        Self {
            lexer: Lexer::new(src),
        }
    }
}

impl<'s> Parser<'s> {
    fn expect_token(&mut self) -> Result<Token<'s>, ParseErr<'s>> {
        match self.lexer.next() {
            Some(Ok(tok)) => Ok(tok),
            Some(Err(lex_err)) => {
                Err(ParseErr {
                    typ: ParseErrTyp::Lex(lex_err.typ),
                    span: lex_err.span,
                })
            }
            None => unreachable!("parser must not continue after Eof (either returns success if eof is okay or failure if expected more)"),
        }
    }

    fn expect_op_kind(tok: Token<'s>) -> Result<OpKind, ParseErr<'s>> {
        match tok.typ {
            TokenTyp::Op(kind) => Ok(kind),
            found => Err(ParseErr {
                typ: ParseErrTyp::ExpectedClass {
                    expected: TokenClass::Op,
                    found,
                },
                span: tok.span,
            }),
        }
    }

    fn expect_typ(expect: TokenTyp, found: Token<'s>) -> Result<(), ParseErr<'s>> {
        if found.typ == expect {
            Ok(())
        } else {
            Err(ParseErr {
                typ: ParseErrTyp::ExpectedTyp {
                    expected: expect,
                    found: found.typ,
                },
                span: found.span,
            })
        }
    }

    fn expect_operand(
        &mut self,
        expected_typ: OperandType,
        last: bool,
    ) -> Result<TaggedCapability, ParseErr<'s>> {
        let try_operand = self.expect_token()?;
        if !last {
            Self::expect_typ(TokenTyp::Comma, self.expect_token()?)?;
        }
        let operand_typ = try_operand.typ.operand_type().ok_or(ParseErr {
            typ: ParseErrTyp::InvalidOperand,
            span: try_operand.span,
        })?;
        if operand_typ != expected_typ {
            return Err(ParseErr {
                typ: ParseErrTyp::OperandTypeMismatch {
                    expected: expected_typ,
                    found: operand_typ,
                },
                span: try_operand.span,
            });
        }
        let tcap = match try_operand.typ {
            TokenTyp::Register(reg) => {
                // register as operand is inlined to its identifying byte representation
                TaggedCapability::from_ugran(reg as _)
            }
            TokenTyp::Syscall(syscall) => {
                // syscall kind as operand is inlined to its byte representation
                TaggedCapability::from_ugran(syscall as _)
            }
            TokenTyp::UnsignedInt(int) => TaggedCapability::from_ugran(int),
            _ => unreachable!(),
        };
        Ok(tcap)
    }

    fn expect_operation(&mut self, start: Token<'s>) -> Result<Op, ParseErr<'s>> {
        // verify that operation starts with OpKind
        let op_kind = Self::expect_op_kind(start)?;

        /* now we expect a variable number of operands to the operation
         * (determined by OpKind::arg_count) */
        let argc = op_kind.operand_count();
        let mut op = Op {
            kind: op_kind,
            op1: TaggedCapability::INVALID,
            op2: TaggedCapability::INVALID,
            op3: TaggedCapability::INVALID,
        };
        let args = [&mut op.op1, &mut op.op2, &mut op.op3];
        for arg in 0..argc {
            let last = arg + 1 == argc;
            let typ = op_kind.type_signature()[arg as usize].unwrap();
            *args[arg as usize] = self.expect_operand(typ, last)?;
        }

        // verify that operation ends with newline
        Self::expect_typ(TokenTyp::Newline, self.expect_token()?)?;

        Ok(op)
    }

    fn next_inner(&mut self) -> Result<Option<Op>, ParseErr<'s>> {
        // skip newlines and handle eof
        let try_op_start = loop {
            let tok = self.expect_token()?;
            match tok.typ {
                TokenTyp::Newline => continue,
                TokenTyp::Eof => return Ok(None),
                _ => break tok,
            }
        };

        // assembly currently only supports lines of operations
        let op = self.expect_operation(try_op_start)?;

        Ok(Some(op))
    }
}

impl<'s> Iterator for Parser<'s> {
    type Item = Result<Op, ParseErr<'s>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_inner().transpose()
    }
}
