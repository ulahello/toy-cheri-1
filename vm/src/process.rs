use tracing::{span, Level};

use crate::access::MemAccessKind;
use crate::exception::Exception;
use crate::mem::Memory;
use crate::op::{Op, OpKind};
use crate::registers::Register;
use crate::syscall::SyscallKind;

impl Memory {
    pub fn execute_op(&mut self) -> Result<(), Exception> {
        let mut pc = self.regs.read(&self.tags, Register::Pc as _).unwrap();
        let op = self.read_op(pc)?;
        pc.check_access(MemAccessKind::Execute, Op::ALIGN, Some(Op::SIZE as _))?;

        let span = span!(
            Level::INFO,
            "exe_op",
            op_kind = op.kind.to_byte(),
            op1 = op.op1.capability().to_ugran(),
            op2 = op.op2.capability().to_ugran(),
            op3 = op.op3.capability().to_ugran(),
            pc = pc.addr().get()
        );
        let _guard = span.enter();

        tracing::trace!("executing {:?}", op);

        match op.kind {
            OpKind::Nop => (),

            OpKind::LoadI => {
                let dst = op.op1;
                let imm = op.op2;
                self.regs.write(
                    &mut self.tags,
                    (dst.capability().to_ugran() & 0xff) as _,
                    imm,
                )?;
            }

            OpKind::Syscall => {
                let kind = self.regs.read(&self.tags, Register::A0 as _)?;
                let kind = SyscallKind::from_byte((kind.capability().to_ugran() & 0xff) as _)?;
                tracing::trace!("syscall {kind:?}");
                match kind {
                    SyscallKind::Exit => return Err(Exception::ProcessExit),
                }
            }
        }

        // increment pc
        pc = pc.set_addr(pc.addr().add(Op::SIZE as _));
        self.regs
            .write(&mut self.tags, Register::Pc as _, pc)
            .unwrap();

        Ok(())
    }
}
