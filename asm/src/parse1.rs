use fruticose_vm::capability::TaggedCapability;
use fruticose_vm::op::OpKind;

use core::iter::Peekable;

use crate::lex::{LexErrTyp, Lexer, Token, TokenTyp};
use crate::Span;

/* TODOO: if `next` last yielded Err, context of call stack is lost so parser
 * may for instance expect that an operand is actually the start of an
 * operation */

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseErr<'s> {
    pub typ: ParseErrTyp<'s>,
    pub span: Span<'s>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrTyp<'s> {
    Lex(LexErrTyp),
    ExpectedTyp {
        expected: TokenTyp,
        found: TokenTyp,
    },
    ExpectedClass {
        expected: TokenClass,
        found: TokenTyp,
    },
    InvalidStmtStart {
        found: TokenTyp,
    },
    InvalidOperand {
        found: TokenTyp,
    },
    OperandTypeMismatch {
        expected: OperandType,
        found: OperandType,
    },
    LabelRedef {
        first_def: Span<'s>,
    },
    LabelUndef,
    LabelOffsetOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenClass {
    Op,
    Register,
    Syscall,
    Literal,
    Identifier,
}

impl TokenTyp {
    pub const fn classify(self) -> Option<TokenClass> {
        match self {
            Self::Op(_) => Some(TokenClass::Op),
            Self::Register(_) => Some(TokenClass::Register),
            Self::Syscall(_) => Some(TokenClass::Syscall),
            Self::UnsignedInt(_) => Some(TokenClass::Literal),
            Self::Identifier => Some(TokenClass::Identifier),
            Self::Comma | Self::Colon | Self::Newline | Self::Eof => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandType {
    Register,
    Immediate,
    Label,
    Unused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandVal<'s> {
    Known(TaggedCapability),
    Ref(Span<'s>),
}

impl<'s> OperandVal<'s> {
    pub const fn unwrap(self) -> TaggedCapability {
        match self {
            Self::Known(val) => val,
            Self::Ref(_) => panic!(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Operand<'s> {
    pub typ: OperandType,
    pub val: Option<OperandVal<'s>>,
}

impl Operand<'static> {
    pub const UNUSED: Self = Self {
        typ: OperandType::Unused,
        val: None,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Label<'s> {
    pub id: Span<'s>,
    pub op_idx: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XOp<'s> {
    pub kind: OpKind,
    pub op1: Operand<'s>,
    pub op2: Operand<'s>,
    pub op3: Operand<'s>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stmt<'s> {
    Label(Label<'s>),
    Op(XOp<'s>),
}

pub const fn type_signature(op: OpKind) -> [Option<OperandType>; 3] {
    const fn sig<const N: usize>(
        op: OpKind,
        sig: [OperandType; N],
    ) -> [Option<OperandType>; OpKind::MAX_OPERANDS] {
        assert!(N <= OpKind::MAX_OPERANDS);
        if sig.len() != op.operand_count() as _ {
            panic!("signature must have correct operand count");
        }
        let mut out = [None; 3];

        let mut idx = 0;
        while idx < sig.len() {
            out[idx] = Some(sig[idx]);
            idx += 1;
        }
        out
    }

    use OperandType::*;

    match op {
        OpKind::CGetValid => sig(op, [Register, Register]),
        OpKind::CGetAddr => sig(op, [Register, Register]),
        OpKind::CSetAddr => sig(op, [Register, Register]),
        OpKind::CGetBound => sig(op, [Register, Register, Register]),
        OpKind::CSetBound => sig(op, [Register, Register, Register]),
        OpKind::CGetPerm => sig(op, [Register, Register]),
        OpKind::CSetPerm => sig(op, [Register, Register]),
        OpKind::CGetType => sig(op, [Register, Register]),
        OpKind::CSeal => sig(op, [Register, Register, Register]),
        OpKind::CUnseal => sig(op, [Register, Register, Register]),
        OpKind::Cpy => sig(op, [Register, Register]),
        OpKind::LoadI => sig(op, [Register, Immediate]),
        OpKind::LoadU8 => sig(op, [Register, Register]),
        OpKind::LoadU16 => sig(op, [Register, Register]),
        OpKind::LoadU32 => sig(op, [Register, Register]),
        OpKind::LoadU64 => sig(op, [Register, Register]),
        OpKind::LoadC => sig(op, [Register, Register]),
        OpKind::Store8 => sig(op, [Register, Register]),
        OpKind::Store16 => sig(op, [Register, Register]),
        OpKind::Store32 => sig(op, [Register, Register]),
        OpKind::Store64 => sig(op, [Register, Register]),
        OpKind::StoreC => sig(op, [Register, Register]),
        OpKind::AddI => sig(op, [Register, Register, Immediate]),
        OpKind::Add => sig(op, [Register, Register, Register]),
        OpKind::Sub => sig(op, [Register, Register, Register]),
        OpKind::SltsI => sig(op, [Register, Register, Immediate]),
        OpKind::SltuI => sig(op, [Register, Register, Immediate]),
        OpKind::Slts => sig(op, [Register, Register, Register]),
        OpKind::Sltu => sig(op, [Register, Register, Register]),
        OpKind::XorI => sig(op, [Register, Register, Immediate]),
        OpKind::Xor => sig(op, [Register, Register, Register]),
        OpKind::OrI => sig(op, [Register, Register, Immediate]),
        OpKind::Or => sig(op, [Register, Register, Register]),
        OpKind::AndI => sig(op, [Register, Register, Immediate]),
        OpKind::And => sig(op, [Register, Register, Register]),
        OpKind::SllI => sig(op, [Register, Register, Immediate]),
        OpKind::Sll => sig(op, [Register, Register, Register]),
        OpKind::SrlI => sig(op, [Register, Register, Immediate]),
        OpKind::Srl => sig(op, [Register, Register, Register]),
        OpKind::SraI => sig(op, [Register, Register, Immediate]),
        OpKind::Sra => sig(op, [Register, Register, Register]),
        OpKind::Jal => sig(op, [Register, Label]),
        OpKind::Jalr => sig(op, [Register, Register, Label]),
        OpKind::Beq => sig(op, [Register, Register, Label]),
        OpKind::Bne => sig(op, [Register, Register, Label]),
        OpKind::Blts => sig(op, [Register, Register, Label]),
        OpKind::Bges => sig(op, [Register, Register, Label]),
        OpKind::Bltu => sig(op, [Register, Register, Label]),
        OpKind::Bgeu => sig(op, [Register, Register, Label]),
        OpKind::Syscall => sig(op, []),
    }
}

pub struct Parser1<'s> {
    lexer: Peekable<Lexer<'s>>,
    op_idx: usize,
}

impl<'s> Parser1<'s> {
    pub fn new(src: &'s str) -> Self {
        Self {
            lexer: Lexer::new(src).peekable(),
            op_idx: 0,
        }
    }
}

impl<'s> Parser1<'s> {
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
    ) -> Result<Operand<'s>, ParseErr<'s>> {
        let try_operand = self.expect_token()?;
        let operand_typ = try_operand.typ.operand_type().ok_or(ParseErr {
            typ: ParseErrTyp::InvalidOperand {
                found: try_operand.typ,
            },
            span: try_operand.span,
        })?;
        if !last {
            Self::expect_typ(TokenTyp::Comma, self.expect_token()?)?;
        }
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
                Some(OperandVal::Known(TaggedCapability::from_ugran(reg as _)))
            }
            TokenTyp::Syscall(syscall) => {
                // syscall kind as operand is inlined to its byte representation
                Some(OperandVal::Known(TaggedCapability::from_ugran(
                    syscall as _,
                )))
            }
            TokenTyp::UnsignedInt(int) => {
                Some(OperandVal::Known(TaggedCapability::from_ugran(int)))
            }
            TokenTyp::Identifier => Some(OperandVal::Ref(try_operand.span)),
            _ => unreachable!(),
        };
        Ok(Operand {
            typ: expected_typ,
            val: tcap,
        })
    }

    fn expect_label(&mut self, ident_span: Span<'s>) -> Result<Label<'s>, ParseErr<'s>> {
        Self::expect_typ(TokenTyp::Colon, self.expect_token()?)?;
        Self::expect_typ(TokenTyp::Newline, self.expect_token()?)?;
        Ok(Label {
            id: ident_span,
            op_idx: self.op_idx,
        })
    }

    fn expect_operation<'f>(&'f mut self, op_kind: OpKind) -> Result<XOp<'s>, ParseErr<'s>> {
        /* we expect a variable number of operands to the operation
         * (determined by OpKind::arg_count) */
        let argc = op_kind.operand_count();
        let mut op = XOp {
            kind: op_kind,
            op1: Operand::UNUSED,
            op2: Operand::UNUSED,
            op3: Operand::UNUSED,
        };
        let args = [&mut op.op1, &mut op.op2, &mut op.op3];
        for arg in 0..argc {
            let last = arg + 1 == argc;
            let typ = type_signature(op_kind)[arg as usize].unwrap();
            *args[arg as usize] = self.expect_operand(typ, last)?;
        }

        // verify that operation ends with newline
        Self::expect_typ(TokenTyp::Newline, self.expect_token()?)?;

        Ok(op)
    }

    fn next_inner(&mut self) -> Result<Option<Stmt<'s>>, ParseErr<'s>> {
        // skip newlines and handle eof
        let try_start = loop {
            if self.lexer.peek().is_none() {
                return Ok(None);
            }
            let tok = self.expect_token()?;
            match tok.typ {
                TokenTyp::Newline => continue,
                TokenTyp::Eof => return Ok(None),
                _ => break tok,
            }
        };

        let stmt = match try_start.typ {
            TokenTyp::Op(op_kind) => Stmt::Op(self.expect_operation(op_kind)?),
            TokenTyp::Identifier => Stmt::Label(self.expect_label(try_start.span)?),
            found => {
                return Err(ParseErr {
                    typ: ParseErrTyp::InvalidStmtStart { found },
                    span: try_start.span,
                })
            }
        };

        Ok(Some(stmt))
    }
}

impl<'s> Iterator for Parser1<'s> {
    type Item = Result<Stmt<'s>, ParseErr<'s>>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.next_inner().transpose();
        if let Some(Ok(Stmt::Op(_))) = item {
            self.op_idx += 1;
        }
        item
    }
}
