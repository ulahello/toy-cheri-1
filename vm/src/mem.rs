use anyhow::{anyhow, Context};
use bitvec::bitbox;
use bitvec::boxed::BitBox;
use bitvec::order::Lsb0;
use tracing::{span, Level};

use crate::abi::{Align, Layout, Ty};
use crate::access::MemAccessKind;
use crate::alloc::{self, Strategy};
use crate::capability::{Address, Capability, Granule, Permissions, TaggedCapability};
use crate::exception::Exception;
use crate::int::{UAddr, UGRAN_SIZE, UNINIT};
use crate::op::Op;
use crate::registers::{Register, Registers};

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
            .checked_mul(Op::LAYOUT.size as _)
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

        tracing::debug!("allocating vm memory");
        let bytes = vec![UNINIT as _; mem_len].into_boxed_slice();
        tracing::debug!("initializing registers");
        let regs = Registers::new();
        tracing::debug!("initializing tag controller");
        let tags = TagController::new(granules).context("failed to create tag controller")?;
        let mut mem = Self {
            mem: bytes,
            regs,
            tags,
        };

        /* instantiate init program */
        // set up root capability
        tracing::debug!("acquiring root capability");
        let root = TaggedCapability {
            capa: Capability::new(
                Address(0),
                Address(0),
                Address(UAddr::try_from(mem_len).expect("converted from UAddr to usize at start of Memory::new, so converting back to UAddr is infallible")),
                Permissions {
                    r: true,
                    w: true,
                    x: true,
                },
            ),
            valid: true,
        };
        tracing::debug!("initializing root allocator");
        let root_alloc = {
            alloc::init(Strategy::Bump, root, &mut mem)
                .context("failed to initialize root allocator")?
        };
        {
            let stats = alloc::stat(root_alloc, &mem)?;
            tracing::trace!(stats.bytes_free, "allocator reports stats");
        }

        // write init program
        tracing::debug!("allocating program");
        let mut pc = alloc::alloc(
            root_alloc,
            Layout {
                size: init_bytes,
                align: TaggedCapability::LAYOUT.align,
            },
            &mut mem,
        )
        .context("failed to allocate program")?
        .set_perms(Permissions {
            r: false,
            w: true,
            x: false,
        });
        tracing::debug!(pc = pc.addr().get(), "writing init program to memory");
        mem.write_slice(pc, init)
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

    pub fn read<T: Ty>(&self, mut src: TaggedCapability) -> Result<T, Exception> {
        let layout = T::LAYOUT;
        src.check_access(MemAccessKind::Read, layout.align, Some(layout.size))?;
        src = src.set_bounds(src.addr(), src.addr().add(T::LAYOUT.size));
        T::read_from_mem(src, self)
    }

    pub fn write<T: Ty>(&mut self, mut dst: TaggedCapability, val: T) -> Result<(), Exception> {
        let layout = T::LAYOUT;
        dst.check_access(MemAccessKind::Write, layout.align, Some(layout.size))?;
        dst = dst.set_bounds(dst.addr(), dst.addr().add(T::LAYOUT.size));
        val.write_to_mem(dst, self)
    }

    pub fn write_slice<T: Ty>(
        &mut self,
        mut dst: TaggedCapability,
        vals: &[T],
    ) -> Result<(), Exception> {
        let layout = T::LAYOUT;

        let mut access = dst.access(MemAccessKind::Write, layout.align, None);
        let len = UAddr::try_from(vals.len())
            .ok()
            .and_then(|size| size.checked_mul(layout.size))
            .ok_or(Exception::InvalidMemAccess { access })?;
        access.len = Some(len);
        dst.check_given_access(access)?;

        for val in vals.iter().copied() {
            self.write(dst, val)?;
            dst = dst.set_addr(dst.addr().add(layout.size));
        }

        Ok(())
    }
}

impl Memory {
    pub(crate) fn read_raw(
        &self,
        src: TaggedCapability,
        layout: Layout,
    ) -> Result<&[u8], Exception> {
        src.check_access(MemAccessKind::Read, layout.align, Some(layout.size))?;
        // casts assume that bounds of capability lie within bounds of self.mem
        let start_idx = usize::try_from(src.addr().get()).unwrap();
        let endb_idx = start_idx + layout.size as usize;
        Ok(&self.mem[start_idx..endb_idx])
    }

    pub(crate) fn write_raw(
        &mut self,
        dst: TaggedCapability,
        align: Align,
        buf: &[u8],
    ) -> Result<(), Exception> {
        let mut access = dst.access(MemAccessKind::Write, align, None);
        let buf_len =
            UAddr::try_from(buf.len()).map_err(|_| Exception::InvalidMemAccess { access })?;
        access.len = Some(buf_len);
        dst.check_given_access(access)?;
        // casts assume that bounds of capability lie within bounds of self.mem
        let start_idx = usize::try_from(dst.addr().get()).unwrap();
        let endb_idx: usize = start_idx + buf_len as usize;
        self.mem[start_idx..endb_idx].copy_from_slice(buf);
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
        *self.mem.get_mut(idx)? = valid;
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
