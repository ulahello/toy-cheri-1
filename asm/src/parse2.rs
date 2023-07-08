use fruticose_vm::capability::TaggedCapability;
use fruticose_vm::int::{gran_unsign, SAddr, SGran};
use fruticose_vm::op::Op;

use core::iter::Enumerate;
use std::collections::HashMap;
use std::vec;

use crate::parse1::{Label, OperandType, OperandVal, ParseErr, ParseErrTyp, Parser1, Stmt, XOp};

pub struct Parser2<'s> {
    xops: Enumerate<vec::IntoIter<XOp<'s>>>,
    labels: HashMap<&'s str, Label<'s>>,
    errs: Vec<ParseErr<'s>>,
}

impl<'s> Parser2<'s> {
    pub fn new(s: &'s str) -> Self {
        let mut xops: Vec<XOp> = Vec::new();
        let mut labels: HashMap<&str, Label<'_>> = HashMap::new();
        let mut errs: Vec<ParseErr<'_>> = Vec::new();

        for stmt in Parser1::new(s) {
            match stmt {
                Ok(Stmt::Op(xop)) => xops.push(xop),
                Ok(Stmt::Label(label)) => {
                    if let Some(old) = labels.insert(label.id.get(), label) {
                        errs.push(ParseErr {
                            typ: ParseErrTyp::LabelRedef { first_def: old.id },
                            span: label.id,
                        });
                    }
                }
                Err(err) => errs.push(err),
            }
        }

        Self {
            xops: xops.into_iter().enumerate(),
            labels,
            errs,
        }
    }

    fn next_inner(&mut self) -> Result<Option<Op>, ParseErr<'s>> {
        if let Some((cur_op_idx, mut xop)) = self.xops.next() {
            let mut op = Op {
                kind: xop.kind,
                op1: TaggedCapability::INVALID,
                op2: TaggedCapability::INVALID,
                op3: TaggedCapability::INVALID,
            };
            let dst = [&mut op.op1, &mut op.op2, &mut op.op3];
            let src = [&mut xop.op1, &mut xop.op2, &mut xop.op3];
            for (src, dst) in src.into_iter().zip(dst) {
                match src.typ {
                    OperandType::Register => *dst = src.val.unwrap().unwrap(),
                    OperandType::Immediate => *dst = src.val.unwrap().unwrap(),
                    OperandType::Label => {
                        let val = match src.val {
                            Some(OperandVal::Known(val)) => val,
                            Some(OperandVal::Ref(lref)) => match self.labels.get(lref.get()) {
                                Some(label) => {
                                    let overflow_err = ParseErr {
                                        typ: ParseErrTyp::LabelOffsetOverflow,
                                        span: lref,
                                    };

                                    let label_op_idx: SAddr = SAddr::try_from(label.op_idx)
                                        .map_err(|_| overflow_err.clone())?;
                                    let cur_op_idx: SAddr = SAddr::try_from(cur_op_idx)
                                        .map_err(|_| overflow_err.clone())?;
                                    let offset: SAddr =
                                        label_op_idx.checked_sub(cur_op_idx).ok_or(overflow_err)?;
                                    TaggedCapability::from_ugran(gran_unsign(offset as SGran))
                                }
                                None => {
                                    return Err(ParseErr {
                                        typ: ParseErrTyp::LabelUndef,
                                        span: lref,
                                    });
                                }
                            },
                            None => unreachable!(),
                        };
                        *dst = val;
                    }
                    OperandType::Unused => (),
                }
            }
            Ok(Some(op))
        } else {
            Ok(None)
        }
    }
}

impl<'s> Iterator for Parser2<'s> {
    type Item = Result<Op, ParseErr<'s>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(err) = self.errs.pop() {
            Some(Err(err))
        } else {
            self.next_inner().transpose()
        }
    }
}
