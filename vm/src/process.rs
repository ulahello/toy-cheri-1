use tracing::{span, Level};

use crate::abi::{Layout, Ty};
use crate::access::MemAccessKind;
use crate::alloc::{self, InitFlags, Strategy};
use crate::capability::{Address, Permissions, TaggedCapability};
use crate::exception::Exception;
use crate::int::{addr_sign, gran_sign, SAddr, SGran, UAddr, UGran};
use crate::mem::Memory;
use crate::op::{Op, OpKind};
use crate::registers::Register;
use crate::syscall::SyscallKind;

impl Memory {
    pub fn execute_next(&mut self) -> Result<(), Exception> {
        let pc = self.regs.read(&self.tags, Register::Pc as _).unwrap();
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

        self.execute_op(op, Some(pc), true)?;

        Ok(())
    }

    pub fn execute_op(
        &mut self,
        op: Op,
        pc: Option<TaggedCapability>,
        bump_pc: bool,
    ) -> Result<(), Exception> {
        let pc = if let Some(cap) = pc {
            cap
        } else {
            self.regs.read(&self.tags, Register::Pc as _)?
        };
        let mut return_address = None; // override return address
        let inc_pc = if bump_pc {
            pc.set_addr(pc.addr().add(Op::LAYOUT.size))
        } else {
            pc
        };
        if bump_pc {
            self.regs
                .write(&mut self.tags, Register::Pc as _, inc_pc)
                .unwrap();
        }

        tracing::trace!("executing {op}");

        match op.kind {
            OpKind::CGetAddr => {
                let dst = reg(op.op1);
                let tcap = self.regs.read(&self.tags, reg(op.op2))?;
                let addr = tcap.addr();
                self.regs.write_ty(&mut self.tags, dst, addr)?;
            }

            OpKind::CSetAddr => {
                let tcap_reg = reg(op.op1);
                let mut tcap = self.regs.read(&self.tags, tcap_reg)?;
                let addr: Address = self.regs.read_ty(&self.tags, reg(op.op2))?;
                tcap = tcap.set_addr(addr);
                self.regs.write(&mut self.tags, tcap_reg, tcap)?;
            }

            OpKind::CGetBound => {
                let start_dst = reg(op.op1);
                let endb_dst = reg(op.op2);
                let tcap = self.regs.read(&self.tags, reg(op.op3))?;
                let start = tcap.start();
                let endb = tcap.endb();
                self.regs.write_ty(&mut self.tags, start_dst, start)?;
                self.regs.write_ty(&mut self.tags, endb_dst, endb)?;
            }

            OpKind::CSetBound => {
                let tcap_reg = reg(op.op1);
                let mut tcap = self.regs.read(&self.tags, tcap_reg)?;
                let start: Address = self.regs.read_ty(&self.tags, reg(op.op2))?;
                let endb: Address = self.regs.read_ty(&self.tags, reg(op.op3))?;
                tcap = tcap.set_bounds(start, endb);
                self.regs.write(&mut self.tags, tcap_reg, tcap)?;
            }

            OpKind::CGetPerm => {
                let dst = reg(op.op1);
                let tcap = self.regs.read(&self.tags, reg(op.op2))?;
                let perms = tcap.perms();
                self.regs.write_ty(&mut self.tags, dst, perms)?;
            }

            OpKind::CSetPerm => {
                let tcap_reg = reg(op.op1);
                let mut tcap = self.regs.read(&self.tags, tcap_reg)?;
                let perms: Permissions = self.regs.read_ty(&self.tags, reg(op.op2))?;
                tcap = tcap.set_perms(perms);
                self.regs.write(&mut self.tags, tcap_reg, tcap)?;
            }

            OpKind::CGetValid => {
                let dst = reg(op.op1);
                let tcap = self.regs.read(&self.tags, reg(op.op2))?;
                self.regs.write_ty(&mut self.tags, dst, tcap.is_valid())?;
            }

            OpKind::Cpy => {
                let dst = reg(op.op1);
                let src = reg(op.op2);
                let val = self.regs.read(&self.tags, src)?;
                self.regs.write(&mut self.tags, dst, val)?;
            }

            OpKind::LoadI => {
                let dst = reg(op.op1);
                let imm = op.op2;
                self.regs.write(&mut self.tags, dst, imm)?;
            }

            OpKind::LoadU8 => {
                let dst = reg(op.op1);
                let src = self.regs.read(&self.tags, reg(op.op2))?;
                let val: u8 = self.read(src)?;
                self.regs.write_ty(&mut self.tags, dst, val)?;
            }

            OpKind::LoadU16 => {
                let dst = reg(op.op1);
                let src = self.regs.read(&self.tags, reg(op.op2))?;
                let val: u16 = self.read(src)?;
                self.regs.write_ty(&mut self.tags, dst, val)?;
            }

            OpKind::LoadU32 => {
                let dst = reg(op.op1);
                let src = self.regs.read(&self.tags, reg(op.op2))?;
                let val: u32 = self.read(src)?;
                self.regs.write_ty(&mut self.tags, dst, val)?;
            }

            OpKind::LoadU64 => {
                let dst = reg(op.op1);
                let src = self.regs.read(&self.tags, reg(op.op2))?;
                let val: u64 = self.read(src)?;
                self.regs.write_ty(&mut self.tags, dst, val)?;
            }

            OpKind::LoadC => {
                let dst = reg(op.op1);
                let src = self.regs.read(&self.tags, reg(op.op2))?;
                let val = self.read(src)?;
                self.regs.write(&mut self.tags, dst, val)?;
            }

            OpKind::Store8 => {
                let dst = self.regs.read(&self.tags, reg(op.op1))?;
                let src = reg(op.op2);
                let val: u8 = self.regs.read_data(src)? as _;
                self.write(dst, val)?;
            }

            OpKind::Store16 => {
                let dst = self.regs.read(&self.tags, reg(op.op1))?;
                let src = reg(op.op2);
                let val: u16 = self.regs.read_data(src)? as _;
                self.write(dst, val)?;
            }

            OpKind::Store32 => {
                let dst = self.regs.read(&self.tags, reg(op.op1))?;
                let src = reg(op.op2);
                let val: u32 = self.regs.read_data(src)? as _;
                self.write(dst, val)?;
            }

            OpKind::Store64 => {
                let dst = self.regs.read(&self.tags, reg(op.op1))?;
                let src = reg(op.op2);
                let val: u64 = self.regs.read_data(src)? as _;
                self.write(dst, val)?;
            }

            OpKind::StoreC => {
                let dst = self.regs.read(&self.tags, reg(op.op1))?;
                let src = reg(op.op2);
                let cap = self.regs.read(&self.tags, src)?;
                self.write(dst, cap)?;
            }

            OpKind::AddI => {
                let dst = reg(op.op1);
                let addend: UGran = self.regs.read_data(reg(op.op2))?;
                let imm: UGran = op.op3.to_ugran();
                let sum = addend.wrapping_add(imm);
                self.regs.write_data(&mut self.tags, dst, sum)?;
            }

            OpKind::Add => {
                let dst = reg(op.op1);
                let add1: UGran = self.regs.read_data(reg(op.op2))?;
                let add2: UGran = self.regs.read_data(reg(op.op3))?;
                let sum = add1.wrapping_add(add2);
                self.regs.write_data(&mut self.tags, dst, sum)?;
            }

            OpKind::Sub => {
                let dst = reg(op.op1);
                let add1: UGran = self.regs.read_data(reg(op.op2))?;
                let add2: UGran = self.regs.read_data(reg(op.op3))?;
                let sum = add1.wrapping_sub(add2);
                self.regs.write_data(&mut self.tags, dst, sum)?;
            }

            OpKind::SltsI => {
                let dst = reg(op.op1);
                let op2: SGran = self.regs.read_ty(&self.tags, reg(op.op2))?;
                let op3: SGran = gran_sign(op.op3.to_ugran());
                self.regs.write_ty(&mut self.tags, dst, op2 < op3)?;
            }

            OpKind::SltuI => {
                let dst = reg(op.op1);
                let op2: UGran = self.regs.read_data(reg(op.op2))?;
                let op3: UGran = op.op3.to_ugran();
                self.regs.write_ty(&mut self.tags, dst, op2 < op3)?;
            }

            OpKind::Slts => {
                let dst = reg(op.op1);
                let op2: SGran = self.regs.read_ty(&self.tags, reg(op.op2))?;
                let op3: SGran = self.regs.read_ty(&self.tags, reg(op.op3))?;
                self.regs.write_ty(&mut self.tags, dst, op2 < op3)?;
            }

            OpKind::Sltu => {
                let dst = reg(op.op1);
                let op2: UGran = self.regs.read_data(reg(op.op2))?;
                let op3: UGran = self.regs.read_data(reg(op.op3))?;
                self.regs.write_ty(&mut self.tags, dst, op2 < op3)?;
            }

            OpKind::XorI => {
                let dst = reg(op.op1);
                let op2: UGran = self.regs.read_data(reg(op.op2))?;
                let op3: UGran = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, op2 ^ op3)?;
            }

            OpKind::Xor => {
                let dst = reg(op.op1);
                let op2: UGran = self.regs.read_data(reg(op.op2))?;
                let op3: UGran = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, op2 ^ op3)?;
            }

            OpKind::OrI => {
                let dst = reg(op.op1);
                let op2: UGran = self.regs.read_data(reg(op.op2))?;
                let op3: UGran = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, op2 | op3)?;
            }

            OpKind::Or => {
                let dst = reg(op.op1);
                let op2: UGran = self.regs.read_data(reg(op.op2))?;
                let op3: UGran = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, op2 | op3)?;
            }

            OpKind::AndI => {
                let dst = reg(op.op1);
                let op2: UGran = self.regs.read_data(reg(op.op2))?;
                let op3: UGran = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, op2 & op3)?;
            }

            OpKind::And => {
                let dst = reg(op.op1);
                let op2: UGran = self.regs.read_data(reg(op.op2))?;
                let op3: UGran = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, op2 & op3)?;
            }

            OpKind::SllI => {
                let dst = reg(op.op1);
                let val: UGran = self.regs.read_data(reg(op.op2))?;
                let amount: UGran = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, val << amount)?;
            }

            OpKind::Sll => {
                let dst = reg(op.op1);
                let val: UGran = self.regs.read_data(reg(op.op2))?;
                let amount: UGran = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, val << amount)?;
            }

            OpKind::SrlI => {
                let dst = reg(op.op1);
                let val: UGran = self.regs.read_data(reg(op.op2))?;
                let amount: UGran = op.op3.to_ugran();
                self.regs.write_data(&mut self.tags, dst, val >> amount)?;
            }

            OpKind::Srl => {
                let dst = reg(op.op1);
                let val: UGran = self.regs.read_data(reg(op.op2))?;
                let amount: UGran = self.regs.read_data(reg(op.op3))?;
                self.regs.write_data(&mut self.tags, dst, val >> amount)?;
            }

            OpKind::SraI => {
                let dst = reg(op.op1);
                let val: SGran = self.regs.read_ty(&self.tags, reg(op.op2))?;
                let amount: UGran = op.op3.to_ugran();
                self.regs.write_ty(&mut self.tags, dst, val >> amount)?;
            }

            OpKind::Sra => {
                let dst = reg(op.op1);
                let val: SGran = self.regs.read_ty(&self.tags, reg(op.op2))?;
                let amount: UGran = self.regs.read_data(reg(op.op3))?;
                self.regs.write_ty(&mut self.tags, dst, val >> amount)?;
            }

            OpKind::Jal => {
                let ra_dst = reg(op.op1);
                let offset: SAddr = addr_sign(op.op2.to_ugran() as UAddr);
                self.regs.write(&mut self.tags, ra_dst, inc_pc)?;
                return_address = Some(pc.set_addr(pc.addr().offset(offset)));
            }

            OpKind::Jalr => {
                let ra_dst = reg(op.op1);
                let base: UAddr = self.regs.read_ty(&self.tags, reg(op.op2))?;
                let offset_imm: SAddr = addr_sign(op.op3.to_ugran() as UAddr);
                self.regs.write(&mut self.tags, ra_dst, inc_pc)?;
                return_address = Some(pc.set_addr(Address(base).offset(offset_imm)));
            }

            OpKind::Beq => {
                let cmp1: UGran = self.regs.read_data(reg(op.op1))?;
                let cmp2: UGran = self.regs.read_data(reg(op.op2))?;
                let offset: SAddr = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 == cmp2 {
                    return_address = Some(pc.set_addr(pc.addr().offset(offset)));
                }
            }

            OpKind::Bne => {
                let cmp1: UGran = self.regs.read_data(reg(op.op1))?;
                let cmp2: UGran = self.regs.read_data(reg(op.op2))?;
                let offset = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 != cmp2 {
                    return_address = Some(pc.set_addr(pc.addr().offset(offset)));
                }
            }

            OpKind::Blts => {
                let cmp1: SGran = self.regs.read_ty(&self.tags, reg(op.op1))?;
                let cmp2: SGran = self.regs.read_ty(&self.tags, reg(op.op2))?;
                let offset: SAddr = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 < cmp2 {
                    return_address = Some(pc.set_addr(pc.addr().offset(offset)));
                }
            }

            OpKind::Bges => {
                let cmp1: SGran = self.regs.read_ty(&self.tags, reg(op.op1))?;
                let cmp2: SGran = self.regs.read_ty(&self.tags, reg(op.op2))?;
                let offset: SAddr = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 >= cmp2 {
                    return_address = Some(pc.set_addr(pc.addr().offset(offset)));
                }
            }

            OpKind::Bltu => {
                let cmp1: UGran = self.regs.read_data(reg(op.op1))?;
                let cmp2: UGran = self.regs.read_data(reg(op.op2))?;
                let offset: SAddr = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 < cmp2 {
                    return_address = Some(pc.set_addr(pc.addr().offset(offset)));
                }
            }

            OpKind::Bgeu => {
                let cmp1: UGran = self.regs.read_data(reg(op.op1))?;
                let cmp2: UGran = self.regs.read_data(reg(op.op2))?;
                let offset: SAddr = addr_sign(op.op3.to_ugran() as UAddr);
                if cmp1 >= cmp2 {
                    return_address = Some(pc.set_addr(pc.addr().offset(offset)));
                }
            }

            OpKind::Syscall => {
                let kind: SyscallKind = self.regs.read_ty(&self.tags, Register::A2 as _)?;

                let span = span!(Level::INFO, "syscall", kind = format_args!("{kind}"));
                let _enter = span.enter();

                tracing::debug!("executing syscall {kind}");

                /* TODOO: allocation failures are currently fatal, but that's
                 * silly. how should a userspace asm program handle allocation
                 * failure, and what does that mean for the allocator api? */

                match kind {
                    SyscallKind::Exit => return Err(Exception::ProcessExit),

                    SyscallKind::AllocInit => {
                        let strategy: Strategy =
                            self.regs.read_ty(&self.tags, Register::A3 as _)?;
                        let flags: InitFlags = self.regs.read_ty(&self.tags, Register::A4 as _)?;
                        let region = self.regs.read(&self.tags, Register::A5 as _)?;
                        tracing::trace!(
                            strategy = format_args!("{strategy:?}"),
                            flags = format_args!("{flags:?}"),
                            region = format_args!("{region:?}"),
                            "initializing allocator"
                        );
                        let ator = alloc::init(strategy, flags, region, self)?;
                        tracing::trace!(ator = format_args!("{ator:?}"), "init ok");
                        self.regs.write(&mut self.tags, Register::A0 as _, ator)?;
                    }

                    SyscallKind::AllocDeInit => {
                        let ator = self.regs.read(&self.tags, Register::A3 as _)?;
                        tracing::trace!(ator = format_args!("{ator:?}"), "requesting de-init");
                        let region = alloc::deinit(ator, self)?;
                        tracing::trace!(
                            region = format_args!("{region:?}"),
                            "de-init ok, reclaimed region"
                        );
                        self.regs.write(&mut self.tags, Register::A0 as _, region)?;
                    }

                    SyscallKind::AllocAlloc => {
                        let ator = self.regs.read(&self.tags, Register::A3 as _)?;
                        let layout: Layout = self.regs.read_ty(&self.tags, Register::A4 as _)?;
                        tracing::trace!(
                            ator = format_args!("{ator:?}"),
                            layout = format_args!("{layout:?}"),
                            "requesting allocation"
                        );
                        let ation = alloc::alloc(ator, layout, self)?;
                        tracing::trace!(ation = format_args!("{ation:?}"), "allocation ok");
                        self.regs.write(&mut self.tags, Register::A0 as _, ation)?;
                    }

                    SyscallKind::AllocFree => {
                        let ator = self.regs.read(&self.tags, Register::A3 as _)?;
                        let ation = self.regs.read(&self.tags, Register::A4 as _)?;
                        tracing::trace!(
                            ator = format_args!("{ator:?}"),
                            ation = format_args!("{ation:?}"),
                            "requesting allocation free"
                        );
                        alloc::free(ator, ation, self)?;
                        tracing::trace!("freeing ok");
                    }

                    SyscallKind::AllocFreeAll => {
                        let ator = self.regs.read(&self.tags, Register::A3 as _)?;
                        tracing::trace!(
                            ator = format_args!("{ator:?}"),
                            "requesting allocator free all"
                        );
                        alloc::free_all(ator, self)?;
                        tracing::trace!("freeing all ok");
                    }

                    SyscallKind::AllocStat => {
                        let ator = self.regs.read(&self.tags, Register::A3 as _)?;
                        tracing::trace!(ator = format_args!("{ator:?}"), "statting allocator");
                        let stats = alloc::stat(ator, self)?;
                        tracing::trace!(stats = format_args!("{stats:?}"), "statting ok");
                        self.regs
                            .write_ty(&mut self.tags, Register::A0 as _, stats)?;
                    }
                }
            }
        }

        // return address was overridden
        if let Some(ra) = return_address {
            self.regs.write(&mut self.tags, Register::Pc as _, ra)?;
        }

        Ok(())
    }
}

fn reg(tcap: TaggedCapability) -> u8 {
    tcap.to_ugran() as u8
}
