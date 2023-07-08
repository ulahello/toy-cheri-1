use tracing::{span, Level};

use crate::abi::Ty;
use crate::access::MemAccessKind;
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::int::{addr_sign, gran_sign, gran_unsign, UAddr};
use crate::mem::Memory;
use crate::op::{Op, OpKind};
use crate::registers::Register;
use crate::syscall::SyscallKind;

impl Memory {
    pub fn execute_op(&mut self) -> Result<(), Exception> {
        let mut pc = self.regs.read(&self.tags, Register::Pc as _).unwrap();
        let op: Op = self.read(pc)?;
        pc.check_access(
            MemAccessKind::Execute,
            Op::LAYOUT.align,
            Some(Op::LAYOUT.size),
        )?;

        let span = span!(
            Level::INFO,
            "exe_op",
            op_kind = op.kind.to_byte(),
            op1 = op.op1.to_ugran(),
            op2 = op.op2.to_ugran(),
            op3 = op.op3.to_ugran(),
            pc = pc.addr().get()
        );
        let _guard = span.enter();

        let mut return_address = pc.set_addr(pc.addr().add(Op::LAYOUT.size));

        tracing::trace!("executing {:?}", op);

        match op.kind {
            OpKind::Nop => (),

            OpKind::LoadI => {
                let dst = reg(op.op1);
                let imm = op.op2;
                self.regs.write(&mut self.tags, dst, imm)?;
            }

            OpKind::LoadU8 => {
                let dst = reg(op.op1);
                let src = op.op2;
                let val: u8 = self.read(src)?;
                self.regs.write_data(&mut self.tags, dst, val.into())?;
            }

            OpKind::LoadU16 => {
                let dst = reg(op.op1);
                let src = op.op2;
                let val: u16 = self.read(src)?;
                self.regs.write_data(&mut self.tags, dst, val.into())?;
            }

            OpKind::LoadU32 => {
                let dst = reg(op.op1);
                let src = op.op2;
                let val: u32 = self.read(src)?;
                self.regs.write_data(&mut self.tags, dst, val.into())?;
            }

            OpKind::LoadU64 => {
                let dst = reg(op.op1);
                let src = op.op2;
                let val: u64 = self.read(src)?;
                self.regs.write_data(&mut self.tags, dst, val.into())?;
            }

            OpKind::LoadU128 => {
                let dst = reg(op.op1);
                let src = op.op2;
                let val: u128 = self.read(src)?;
                self.regs.write_data(&mut self.tags, dst, val)?;
            }

            OpKind::LoadC => {
                let dst = reg(op.op1);
                let src = op.op2;
                let cap: TaggedCapability = self.read(src)?;
                self.regs.write(&mut self.tags, dst, cap)?;
            }

            OpKind::Store8 => {
                let dst = op.op1;
                let src = reg(op.op2);
                let val: u8 = self.regs.read_data(src)? as _;
                self.write(dst, val)?;
            }

            OpKind::Store16 => {
                let dst = op.op1;
                let src = reg(op.op2);
                let val: u16 = self.regs.read_data(src)? as _;
                self.write(dst, val)?;
            }

            OpKind::Store32 => {
                let dst = op.op1;
                let src = reg(op.op2);
                let val: u32 = self.regs.read_data(src)? as _;
                self.write(dst, val)?;
            }

            OpKind::Store64 => {
                let dst = op.op1;
                let src = reg(op.op2);
                let val: u64 = self.regs.read_data(src)? as _;
                self.write(dst, val)?;
            }

            OpKind::Store128 => {
                let dst = op.op1;
                let src = reg(op.op2);
                let val: u128 = self.regs.read_data(src)? as _;
                self.write(dst, val)?;
            }

            OpKind::StoreC => {
                let dst = op.op1;
                let src = reg(op.op2);
                let cap = self.regs.read(&self.tags, src)?;
                self.write(dst, cap)?;
            }

            OpKind::AddI => {
                let dst = reg(op.op1);
                let addend = self.regs.read_data(reg(op.op2))?;
                let imm = op.op3.to_ugran();
                let sum = addend.wrapping_add(imm);
                self.regs.write_data(&mut self.tags, dst, sum)?;
            }

            OpKind::Add => {
                let dst = reg(op.op1);
                let add1 = self.regs.read_data(reg(op.op2))?;
                let add2 = self.regs.read_data(reg(op.op3))?;
                let sum = add1.wrapping_add(add2);
                self.regs.write_data(&mut self.tags, dst, sum)?;
            }

            OpKind::Sub => {
                let dst = reg(op.op1);
                let add1 = self.regs.read_data(reg(op.op2))?;
                let add2 = self.regs.read_data(reg(op.op3))?;
                let sum = add1.wrapping_sub(add2);
                self.regs.write_data(&mut self.tags, dst, sum)?;
            }

            OpKind::SltsI => {
                let dst = reg(op.op1);
                let op2 = gran_sign(self.regs.read_data(reg(op.op2))?);
                let op3 = gran_sign(op.op3.to_ugran());
                self.regs
                    .write_data(&mut self.tags, dst, (op2 < op3) as _)?;
            }

            OpKind::SltuI => {
                let dst = reg(op.op1);
                let op2 = self.regs.read_data(reg(op.op2))?;
                let op3 = op.op3.to_ugran();
                self.regs
                    .write_data(&mut self.tags, dst, (op2 < op3) as _)?;
            }

            OpKind::Slts => {
                let dst = reg(op.op1);
                let op2 = gran_sign(self.regs.read_data(reg(op.op2))?);
                let op3 = gran_sign(self.regs.read_data(reg(op.op3))?);
                self.regs
                    .write_data(&mut self.tags, dst, (op2 < op3) as _)?;
            }

            OpKind::Sltu => {
                let dst = reg(op.op1);
                let op2 = self.regs.read_data(reg(op.op2))?;
                let op3 = self.regs.read_data(reg(op.op3))?;
                self.regs
                    .write_data(&mut self.tags, dst, (op2 < op3) as _)?;
            }

            OpKind::XorI => {
                let dst = reg(op.op1);
                let op2 = self.regs.read_data(reg(op.op2))?;
                let op3 = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, op2 ^ op3)?;
            }

            OpKind::Xor => {
                let dst = reg(op.op1);
                let op2 = self.regs.read_data(reg(op.op2))?;
                let op3 = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, op2 ^ op3)?;
            }

            OpKind::OrI => {
                let dst = reg(op.op1);
                let op2 = self.regs.read_data(reg(op.op2))?;
                let op3 = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, op2 | op3)?;
            }

            OpKind::Or => {
                let dst = reg(op.op1);
                let op2 = self.regs.read_data(reg(op.op2))?;
                let op3 = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, op2 | op3)?;
            }

            OpKind::AndI => {
                let dst = reg(op.op1);
                let op2 = self.regs.read_data(reg(op.op2))?;
                let op3 = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, op2 & op3)?;
            }

            OpKind::And => {
                let dst = reg(op.op1);
                let op2 = self.regs.read_data(reg(op.op2))?;
                let op3 = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, op2 & op3)?;
            }

            OpKind::SllI => {
                let dst = reg(op.op1);
                let val = self.regs.read_data(reg(op.op2))?;
                let amount = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, val << amount)?;
            }

            OpKind::Sll => {
                let dst = reg(op.op1);
                let val = self.regs.read_data(reg(op.op2))?;
                let amount = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, val << amount)?;
            }

            OpKind::SrlI => {
                let dst = reg(op.op1);
                let val = self.regs.read_data(reg(op.op2))?;
                let amount = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, val >> amount)?;
            }

            OpKind::Srl => {
                let dst = reg(op.op1);
                let val = self.regs.read_data(reg(op.op2))?;
                let amount = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, val >> amount)?;
            }

            OpKind::SraI => {
                let dst = reg(op.op1);
                let val = gran_sign(self.regs.read_data(reg(op.op2))?);
                let amount = op.op3.to_ugran();
                self.regs
                    .write_data(&mut self.tags, dst, gran_unsign(val >> amount))?;
            }

            OpKind::Sra => {
                let dst = reg(op.op1);
                let val = gran_sign(self.regs.read_data(reg(op.op2))?);
                let amount = self.regs.read_data(reg(op.op3))?;
                self.regs
                    .write_data(&mut self.tags, dst, gran_unsign(val >> amount))?;
            }

            OpKind::Jal => {
                let ra_dst = reg(op.op1);
                let offset = addr_sign(op.op2.to_ugran() as UAddr);
                self.regs.write(&mut self.tags, ra_dst, return_address)?;
                return_address =
                    pc.set_addr(pc.addr().offset(offset.wrapping_mul(Op::LAYOUT.size as _)));
            }

            OpKind::Jalr => {
                let ra_dst = reg(op.op1);
                let offset_reg = addr_sign(self.regs.read_data(reg(op.op2))? as UAddr);
                let offset_imm = addr_sign(op.op3.to_ugran() as UAddr);
                let offset = offset_reg.wrapping_add(offset_imm);
                self.regs.write(&mut self.tags, ra_dst, return_address)?;
                return_address =
                    pc.set_addr(pc.addr().offset(offset.wrapping_mul(Op::LAYOUT.size as _)));
            }

            OpKind::Beq => {
                let cmp1 = self.regs.read_data(reg(op.op1))?;
                let cmp2 = self.regs.read_data(reg(op.op2))?;
                let offset = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 == cmp2 {
                    return_address =
                        pc.set_addr(pc.addr().offset(offset.wrapping_mul(Op::LAYOUT.size as _)));
                }
            }

            OpKind::Bne => {
                let cmp1 = self.regs.read_data(reg(op.op1))?;
                let cmp2 = self.regs.read_data(reg(op.op2))?;
                let offset = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 != cmp2 {
                    return_address =
                        pc.set_addr(pc.addr().offset(offset.wrapping_mul(Op::LAYOUT.size as _)));
                }
            }

            OpKind::Blts => {
                let cmp1 = gran_sign(self.regs.read_data(reg(op.op1))?);
                let cmp2 = gran_sign(self.regs.read_data(reg(op.op2))?);
                let offset = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 < cmp2 {
                    return_address =
                        pc.set_addr(pc.addr().offset(offset.wrapping_mul(Op::LAYOUT.size as _)));
                }
            }

            OpKind::Bges => {
                let cmp1 = gran_sign(self.regs.read_data(reg(op.op1))?);
                let cmp2 = gran_sign(self.regs.read_data(reg(op.op2))?);
                let offset = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 >= cmp2 {
                    return_address =
                        pc.set_addr(pc.addr().offset(offset.wrapping_mul(Op::LAYOUT.size as _)));
                }
            }

            OpKind::Bltu => {
                let cmp1 = self.regs.read_data(reg(op.op1))?;
                let cmp2 = self.regs.read_data(reg(op.op2))?;
                let offset = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 < cmp2 {
                    return_address =
                        pc.set_addr(pc.addr().offset(offset.wrapping_mul(Op::LAYOUT.size as _)));
                }
            }

            OpKind::Bgeu => {
                let cmp1 = self.regs.read_data(reg(op.op1))?;
                let cmp2 = self.regs.read_data(reg(op.op2))?;
                let offset = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 >= cmp2 {
                    return_address =
                        pc.set_addr(pc.addr().offset(offset.wrapping_mul(Op::LAYOUT.size as _)));
                }
            }

            OpKind::Syscall => {
                let kind = self.regs.read(&self.tags, Register::A0 as _)?;
                let kind = SyscallKind::from_byte(kind.to_ugran() as u8)?;
                tracing::trace!("syscall {kind:?}");
                match kind {
                    SyscallKind::Exit => return Err(Exception::ProcessExit),
                }
            }
        }

        // increment pc
        pc = return_address;
        self.regs
            .write(&mut self.tags, Register::Pc as _, pc)
            .unwrap();

        Ok(())
    }
}

fn reg(tcap: TaggedCapability) -> u8 {
    tcap.capability().to_ugran() as u8
}
