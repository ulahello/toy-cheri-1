use fruticose_vm::capability::TaggedCapability;
use fruticose_vm::op::{Op, OpKind};

use crate::lex::{LexErrTyp, Lexer, Token, TokenTyp};
use crate::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseErr<'s> {
    pub typ: ParseErrTyp,
    pub span: Span<'s>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrTyp {
    Lex(LexErrTyp),
    ExpectOp { found: TokenTyp },
    MissingNewline { found: TokenTyp },
    MissingComma { found: TokenTyp },
    InvalidOperand,
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
                return Err(ParseErr {
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
                typ: ParseErrTyp::ExpectOp { found },
                span: tok.span,
            }),
        }
    }

    fn expect_newline(tok: Token<'s>) -> Result<(), ParseErr<'s>> {
        match tok.typ {
            TokenTyp::Newline => Ok(()),
            found => Err(ParseErr {
                typ: ParseErrTyp::MissingNewline { found },
                span: tok.span,
            }),
        }
    }

    fn expect_comma(tok: Token<'s>) -> Result<(), ParseErr<'s>> {
        match tok.typ {
            TokenTyp::Comma => Ok(()),
            found => Err(ParseErr {
                typ: ParseErrTyp::MissingComma { found },
                span: tok.span,
            }),
        }
    }

    fn expect_operand(&mut self, last: bool) -> Result<TaggedCapability, ParseErr<'s>> {
        let try_operand = self.expect_token()?;
        if !last {
            Self::expect_comma(self.expect_token()?)?;
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
            _ => {
                return Err(ParseErr {
                    typ: ParseErrTyp::InvalidOperand,
                    span: try_operand.span,
                })
            }
        };
        Ok(tcap)
    }

    fn expect_operation(&mut self, start: Token<'s>) -> Result<Op, ParseErr<'s>> {
        // verify that operation starts with OpKind
        let op_kind = Self::expect_op_kind(start)?;

        /* now we expect a variable number of operands to the operation
         * (determined by OpKind::arg_count) */
        let argc = op_kind.operand_count();

        let op: Op = match argc {
            0 => Op {
                kind: op_kind,
                op1: TaggedCapability::INVALID,
                op2: TaggedCapability::INVALID,
                op3: TaggedCapability::INVALID,
            },
            1 => Op {
                kind: op_kind,
                op1: self.expect_operand(true)?,
                op2: TaggedCapability::INVALID,
                op3: TaggedCapability::INVALID,
            },
            2 => Op {
                kind: op_kind,
                op1: self.expect_operand(false)?,
                op2: self.expect_operand(true)?,
                op3: TaggedCapability::INVALID,
            },
            3 => Op {
                kind: op_kind,
                op1: self.expect_operand(false)?,
                op2: self.expect_operand(false)?,
                op3: self.expect_operand(true)?,
            },
            4.. => unreachable!("operations have at most 3 operands"),
        };

        // verify that operation ends with newline
        Self::expect_newline(self.expect_token()?)?;

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
