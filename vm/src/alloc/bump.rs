use super::{AllocErr, AllocErrKind, Stats, Strategy};
use crate::abi::{Fields, Layout, Ty};
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::int::UAddr;
use crate::mem::Memory;

#[derive(Clone, Copy, Debug)]
pub(crate) struct BumpAlloc {
    // start: region start
    // addr: first free address (grows upward) (yes ik bump allocs grow downward) or endb
    // endb: region endb
    inner: TaggedCapability,
}

impl BumpAlloc {
    const FIELDS: &'static [Layout] = &[TaggedCapability::LAYOUT, UAddr::LAYOUT];

    pub const fn new(region: TaggedCapability) -> Self {
        Self {
            inner: region.set_addr(region.start()),
        }
    }

    pub const fn stat(&self) -> Stats {
        Stats {
            strategy: Strategy::Bump,
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

    pub fn alloc(&mut self, layout: Layout) -> Result<TaggedCapability, AllocErr> {
        if self.inner.addr() == self.inner.endb() {
            return Err(AllocErr {
                kind: AllocErrKind::Oom,
            });
        }
        let mut ation = self.inner;
        ation = ation.set_addr(ation.addr().align_to(layout.align));
        ation = ation.set_bounds(ation.addr(), ation.addr().add(layout.size));
        if ation.is_valid() {
            self.inner = self.inner.set_addr(ation.endb());
            Ok(ation)
        } else {
            Err(AllocErr {
                kind: AllocErrKind::NotEnoughMem,
            })
        }
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
