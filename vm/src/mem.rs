use anyhow::{anyhow, Context};
use bitvec::bitbox;
use bitvec::boxed::BitBox;
use bitvec::order::Lsb0;
use tracing::{span, Level};

use crate::abi::{Align, Layout, Ty};
use crate::access::MemAccessKind;
use crate::alloc::{self, InitFlags, Strategy};
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
    pub(crate) root: TaggedCapability,
}

impl Memory {
    pub fn new<'op, I: Iterator<Item = &'op Op> + ExactSizeIterator>(
        granules: UAddr,
        init: I,
    ) -> anyhow::Result<Self> {
        fn log_stats(ator: TaggedCapability, mem: &Memory) -> anyhow::Result<()> {
            let stats = alloc::stat(ator, mem).context("failed to stat allocator")?;
            tracing::trace!(
                stats.strategy = format_args!("{:?}", stats.strategy),
                stats.flags = format_args!("{:?}", stats.flags),
                stats.bytes_free,
                "allocator reports stats"
            );
            Ok(())
        }

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
            root: TaggedCapability::INVALID,
        };

        /* instantiate init program */
        // set up root capability
        tracing::debug!("acquiring root capability");
        mem.root = TaggedCapability {
            capa: Capability::new(
                Address(0),
                Address(0),
                Address(UAddr::try_from(mem_len).expect("converted from UAddr to usize at start of Memory::new, so converting back to UAddr is infallible")),
                Permissions::READ | Permissions::WRITE | Permissions::EXEC,
            ),
            valid: true,
        };
        tracing::debug!("initializing root allocator");
        let root_alloc = {
            alloc::init(
                Strategy::Bump,
                InitFlags::INIT_ON_FREE | InitFlags::INIT_ON_ALLOC,
                mem.root,
                &mut mem,
            )
            .context("failed to initialize root allocator")?
        };
        log_stats(root_alloc, &mem)?;

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
        .set_perms(Permissions::WRITE);
        log_stats(root_alloc, &mem)?;
        tracing::debug!(pc = pc.addr().get(), "writing init program to memory");
        mem.write_iter(pc, init)
            .context("failed to write init program to root address")?;

        // remove write access
        pc = pc.set_perms_from(Permissions::READ | Permissions::EXEC, mem.root);
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

    pub fn write_iter<'elem, T: Ty + 'elem, I: Iterator<Item = &'elem T> + ExactSizeIterator>(
        &mut self,
        mut dst: TaggedCapability,
        vals: I,
    ) -> Result<(), Exception> {
        let layout = T::LAYOUT;

        let mut access = dst.access(MemAccessKind::Write, layout.align, None);
        let len = UAddr::try_from(vals.len())
            .ok()
            .and_then(|size| size.checked_mul(layout.size))
            .ok_or(Exception::InvalidMemAccess { access })?;
        access.len = Some(len);
        dst.check_given_access(access)?;

        for val in vals.copied() {
            self.write(dst, val)?;
            dst = dst.set_addr(dst.addr().add(layout.size));
        }

        Ok(())
    }

    pub fn memset(
        &mut self,
        dst: TaggedCapability,
        count: UAddr,
        byte: u8,
    ) -> Result<(), Exception> {
        let mut access = dst.access(MemAccessKind::Write, u8::LAYOUT.align, None);
        access.len = Some(count);
        dst.check_given_access(access)?;

        // casts assume that bounds of capability lie within bounds of self.mem
        let start_idx = dst.addr().get() as usize;
        let dst_slice = &mut self.mem[start_idx..][..count as usize];
        dst_slice.fill(byte);
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
        let start_idx = src.addr().get() as usize;
        Ok(&self.mem[start_idx..][..layout.size as usize])
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
        let start_idx = dst.addr().get() as usize;
        self.mem[start_idx..][..buf_len as usize].copy_from_slice(buf);
        Ok(())
    }
}

#[derive(Debug)]
pub struct TagController {
    // 0..32 => registers
    // 32.. => mem granules
    pub(crate) mem: BitBox<u8, Lsb0>,
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

    pub(crate) fn idx_to_gran(idx: usize) -> Option<Granule> {
        idx.checked_sub(Registers::COUNT as _)
            .and_then(|gran| UAddr::try_from(gran).ok())
            .map(Granule)
    }

    const fn reg_to_idx(reg: u8) -> Option<usize> {
        if Registers::is_reg_valid(reg) {
            Some(reg as _)
        } else {
            None
        }
    }
}
