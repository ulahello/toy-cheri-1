use super::{AllocErr, AllocErrKind, Header, InitFlags, Stats};
use crate::abi::{Fields, Layout, Ty};
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::int::{UAddr, UNINIT_BYTE};
use crate::mem::Memory;
use crate::revoke;

#[derive(Clone, Copy, Debug)]
pub(super) struct BumpAlloc {
    // start: region start
    // addr: first free address (grows upward) (yes ik bump allocs grow downward) or endb
    // endb: region endb
    pub(super) inner: TaggedCapability,
}

impl BumpAlloc {
    const FIELDS: &'static [Layout] = &[TaggedCapability::LAYOUT];

    pub const fn new(region: TaggedCapability) -> Self {
        Self {
            inner: region.set_addr(region.start()),
        }
    }

    pub const fn stat(&self, header: Header) -> Stats {
        Stats {
            strategy: header.strat,
            flags: header.flags,
            bytes_free: self.bytes_free(),
        }
    }

    pub const fn bytes_free(&self) -> UAddr {
        self.inner
            .endb()
            .get()
            .checked_sub(self.inner.addr().get())
            .expect("address must not exceed endb (but can be equal)")
    }

    pub fn alloc(&mut self, header: Header, layout: Layout) -> Result<TaggedCapability, AllocErr> {
        let err = |kind: AllocErrKind| AllocErr {
            stats: self.stat(header),
            requested: layout,
            kind,
        };
        if self.inner.addr() == self.inner.endb() {
            return Err(err(AllocErrKind::Oom));
        }
        let mut ation = self.inner;
        ation = ation.set_addr(ation.addr().align_to(layout.align));
        ation = ation.set_bounds(ation.addr(), ation.addr().add(layout.size));
        if ation.is_valid() {
            self.inner = self.inner.set_addr(ation.endb());
            Ok(ation)
        } else {
            Err(err(AllocErrKind::NotEnoughMem))
        }
    }

    pub fn free_all(&mut self) {
        self.inner = self.inner.set_addr(self.inner.start());
    }
}

impl Ty for BumpAlloc {
    const LAYOUT: Layout = Fields::layout(Self::FIELDS);

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let mut fields = Fields::new(src, Self::FIELDS);
        let inner = fields.next().unwrap();
        Ok(Self {
            inner: TaggedCapability::read_from_mem(inner, mem)?,
        })
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        let mut fields = Fields::new(dst, Self::FIELDS);
        let inner = fields.next().unwrap();
        self.inner.write_to_mem(inner, mem)?;
        Ok(())
    }
}
