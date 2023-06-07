use anyhow::{anyhow, Context};
use bitvec::bitbox;
use bitvec::boxed::BitBox;
use bitvec::order::Lsb0;
use tracing::{span, Level};

use crate::access::{MemAccess, MemAccessKind};
use crate::capability::{Address, Capability, Granule, Permissions, TaggedCapability};
use crate::exception::Exception;
use crate::int::{UAddr, UGran, UADDR_SIZE, UGRAN_SIZE, UNINIT};
use crate::op::{Op, OpKind};
use crate::registers::Register;
use crate::registers::Registers;

#[derive(Debug)]
pub struct Memory {
    pub(crate) mem: Box<[u8]>,
    pub(crate) regs: Registers,
    pub(crate) tags: TagController,
}

impl Memory {
    pub fn new(granules: UAddr, init: &[Op]) -> anyhow::Result<Self> {
        let mem_len = granules
            .checked_mul(UAddr::from(UGRAN_SIZE))
            .and_then(|len| usize::try_from(len).ok())
            .ok_or(anyhow!("allocated bytes overflow"))?;
        let init_elems =
            UAddr::try_from(init.len()).map_err(|_| anyhow!("program length overflow"))?;
        let init_bytes = init_elems
            .checked_mul(Op::SIZE as _)
            .ok_or(anyhow!("program size overflow"))?;

        /* initialize components */
        let span = span!(
            Level::TRACE,
            "mem_init",
            mem_len,
            init_program_len = init_elems,
            init_program_bytes = init_bytes,
        );
        let _guard = span.enter();

        tracing::trace!("allocating vm memory");
        let bytes = vec![UNINIT as _; mem_len].into_boxed_slice();
        tracing::trace!("initializing registers");
        let regs = Registers::new();
        tracing::trace!("initializing tag controller");
        let tags = TagController::new(granules).context("failed to create tag controller")?;
        let mut mem = Self {
            mem: bytes,
            regs,
            tags,
        };

        /* instantiate init program */
        // set up root capability
        // TODO: get root capability from allocator
        tracing::trace!("acquiring root capability");
        let root = TaggedCapability {
            capa: Capability::new(
                 Address(0),
                 Address(0),
                Address(mem_len.try_into().expect("converted from UAddr to usize at start of Memory::new, so converting back to UAddr is infallible")),
                Permissions {
                    r: true,
                    w: true,
                    x: true,
                },
            ),
            valid: true,
        };
        mem.regs
            .write(&mut mem.tags, Register::Z0 as _, root)
            .unwrap();

        // write init program
        let mut pc = root
            .set_bounds(root.start(), root.start().add(init_bytes))
            .set_perms(Permissions {
                r: false,
                w: true,
                x: false,
            });
        tracing::trace!(pc = pc.addr().get(), "writing init program to memory");
        mem.write_ops(pc, init)
            .context("failed to write init program to root address")?;

        // remove write access
        pc = pc.set_perms_from(
            Permissions {
                r: true,
                w: false,
                x: true,
            },
            root,
        );
        mem.regs
            .write(&mut mem.tags, Register::Pc as _, pc)
            .unwrap();

        Ok(mem)
    }

    pub fn read(&self, src: TaggedCapability, len: UAddr) -> Result<&[u8], Exception> {
        src.check_access(MemAccessKind::Read, Some(len))?;
        let start_idx = usize::try_from(src.addr().get()).unwrap();
        let endb_idx = start_idx + len as usize;
        Ok(&self.mem[start_idx..endb_idx])
    }

    pub fn write(&mut self, dst: TaggedCapability, buf: &[u8]) -> Result<(), Exception> {
        let mut access = MemAccess {
            tcap: dst,
            len: None,
            kind: MemAccessKind::Write,
        };
        let buf_len =
            UAddr::try_from(buf.len()).map_err(|_| Exception::InvalidMemAccess { access })?;
        access.len = Some(buf_len);
        dst.check_given_access(access)?;
        let start_idx: usize = dst.addr().get().try_into().unwrap();
        let endb_idx: usize = start_idx + buf_len as usize;
        self.mem[start_idx..endb_idx].copy_from_slice(buf);
        Ok(())
    }

    pub fn read_byte(&self, src: TaggedCapability) -> Result<u8, Exception> {
        let buf = self.read(src, 1)?;
        let byte = u8::from_le_bytes(buf.try_into().unwrap());
        Ok(byte)
    }

    pub fn write_byte(&mut self, dst: TaggedCapability, byte: u8) -> Result<(), Exception> {
        self.write(dst, &byte.to_le_bytes())
    }

    pub fn read_uaddr(&self, src: TaggedCapability) -> Result<UAddr, Exception> {
        let buf = self.read(src, UADDR_SIZE as _)?;
        let val = UAddr::from_le_bytes(buf.try_into().unwrap());
        Ok(val)
    }

    pub fn write_uaddr(&mut self, dst: TaggedCapability, val: UAddr) -> Result<(), Exception> {
        self.write(dst, &val.to_le_bytes())
    }

    pub fn read_ugran(&self, src: TaggedCapability) -> Result<UGran, Exception> {
        let buf = self.read(src, UGRAN_SIZE as _)?;
        let val = UGran::from_le_bytes(buf.try_into().unwrap());
        Ok(val)
    }

    pub fn write_ugran(&mut self, dst: TaggedCapability, val: UGran) -> Result<(), Exception> {
        self.write(dst, &val.to_le_bytes())
    }

    pub fn read_tcap(&self, src: TaggedCapability) -> Result<TaggedCapability, Exception> {
        let data = self.read_ugran(src)?;
        let valid = self
            .tags
            .read_gran(src.addr().gran())
            .expect("read succeeded so address is valid");
        Ok(TaggedCapability {
            capa: Capability::from_ugran(data),
            valid,
        })
    }

    pub fn write_tcap(
        &mut self,
        dst: TaggedCapability,
        tcap: TaggedCapability,
    ) -> Result<(), Exception> {
        let data = tcap.capability().to_ugran();
        self.write(dst, &data.to_le_bytes())?;
        /* now that we've written the data, we need to update the tag controller
         * to preserve validity of capability */
        self.tags
            .write_gran(dst.addr().gran(), tcap.is_valid())
            .expect("valid address must be present in tag controller");
        Ok(())
    }

    pub fn read_op(&self, mut src: TaggedCapability) -> Result<Op, Exception> {
        src.check_access(MemAccessKind::Read, Some(Op::SIZE as _))?;

        let kind = OpKind::from_byte(self.read_byte(src).unwrap())?;
        src.capa.addr.0 += 1;

        let op1 = self.read_tcap(src).unwrap();
        src.capa.addr.0 += UGRAN_SIZE as UAddr;

        let op2 = self.read_tcap(src).unwrap();
        src.capa.addr.0 += UGRAN_SIZE as UAddr;

        let op3 = self.read_tcap(src).unwrap();
        src.capa.addr.0 += UGRAN_SIZE as UAddr;

        Ok(Op {
            kind,
            op1,
            op2,
            op3,
        })
    }

    pub fn write_op(&mut self, mut dst: TaggedCapability, op: Op) -> Result<(), Exception> {
        dst.check_access(MemAccessKind::Write, Some(Op::SIZE as _))?;

        self.write_byte(dst, op.kind.to_byte()).unwrap();
        dst.capa.addr.0 += 1;

        self.write_tcap(dst, op.op1).unwrap();
        dst.capa.addr.0 += UGRAN_SIZE as UAddr;

        self.write_tcap(dst, op.op2).unwrap();
        dst.capa.addr.0 += UGRAN_SIZE as UAddr;

        self.write_tcap(dst, op.op3).unwrap();
        dst.capa.addr.0 += UGRAN_SIZE as UAddr;

        Ok(())
    }

    pub fn write_ops(&mut self, mut dst: TaggedCapability, ops: &[Op]) -> Result<(), Exception> {
        let mut access = dst.access(MemAccessKind::Write, None);
        let ops_size = UAddr::try_from(ops.len())
            .ok()
            .and_then(|len| len.checked_mul(Op::SIZE as _))
            .ok_or(Exception::InvalidMemAccess { access })?;
        access.len = Some(ops_size);
        dst.check_given_access(access)?;

        for op in ops {
            self.write_op(dst, *op).unwrap();
            dst.capa.addr.0 += Op::SIZE as UAddr;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct TagController {
    // 0..32 => registers
    // 32.. => mem granules
    mem: BitBox<u8, Lsb0>,
}

impl TagController {
    pub fn new(granules: UAddr) -> anyhow::Result<Self> {
        let elems = usize::try_from(granules)
            .ok()
            .and_then(|elems| elems.checked_add(Registers::COUNT as _))
            .ok_or(anyhow!("tag count overflow"))?;
        let mut mem = bitbox![_, _; 0; elems];
        debug_assert_eq!(mem.len(), elems);
        // initialize all as invalid
        mem[..].fill(false);
        Ok(Self { mem })
    }

    pub fn read_gran(&self, gran: Granule) -> Option<bool> {
        let idx = Self::gran_to_idx(gran)?;
        Some(self.mem[idx])
    }

    pub fn write_gran(&mut self, gran: Granule, valid: bool) -> Option<()> {
        let idx = Self::gran_to_idx(gran)?;
        *self.mem.get_mut(idx).unwrap() = valid;
        Some(())
    }

    pub fn read_reg(&self, reg: u8) -> Option<bool> {
        let idx = Self::reg_to_idx(reg)?;
        self.mem.get(idx).map(|bit| *bit)
    }

    pub fn write_reg(&mut self, reg: u8, valid: bool) -> Option<()> {
        let idx = Self::reg_to_idx(reg)?;
        if let Some(mut bit) = self.mem.get_mut(idx) {
            *bit = valid;
            Some(())
        } else {
            None
        }
    }
}

impl TagController {
    fn gran_to_idx(gran: Granule) -> Option<usize> {
        let idx = gran.0.checked_add(Registers::COUNT as _)?;
        usize::try_from(idx).ok()
    }

    const fn reg_to_idx(reg: u8) -> Option<usize> {
        if Registers::is_reg_valid(reg) {
            Some(reg as _)
        } else {
            None
        }
    }
}
