mod bump;

use bump::BumpAlloc;

use crate::abi::{Align, Fields, Layout, Ty};
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::int::UAddr;
use crate::mem::Memory;

/* TODO: temporal safety (this will do that allegedly)
explore the following:
- CHERIvoke
- ViK: practical mitigation of temporal memory safety violations through object ID inspection
 */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Strategy {
    Bump = 1,
    // LinkedList,
    // Stack,
}

impl Strategy {
    pub const SIZE: u8 = 1;

    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    pub const fn from_byte(byte: u8) -> Result<Self, Exception> {
        match byte {
            1 => Ok(Self::Bump),
            _ => Err(Exception::InvalidAllocStrategy { byte }),
        }
    }
}

impl Ty for Strategy {
    const LAYOUT: Layout = Layout {
        size: 1,
        align: Align::new(1).unwrap(),
    };

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        Self::from_byte(mem.read(src)?)
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        mem.write(dst, self.to_byte())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub strategy: Strategy,
    pub bytes_free: UAddr,
}

impl Stats {
    const FIELDS: &'static [Layout] = &[Strategy::LAYOUT, UAddr::LAYOUT];
}

impl Ty for Stats {
    const LAYOUT: Layout = Fields::layout(Self::FIELDS);

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let mut fields = Fields::new(src, Self::FIELDS);
        let strategy = fields.next().unwrap();
        let bytes_free = fields.next().unwrap();
        Ok(Self {
            strategy: Strategy::read_from_mem(strategy, mem)?,
            bytes_free: UAddr::read_from_mem(bytes_free, mem)?,
        })
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        let mut fields = Fields::new(dst, Self::FIELDS);
        let strategy = fields.next().unwrap();
        let bytes_free = fields.next().unwrap();
        self.strategy.write_to_mem(strategy, mem)?;
        self.bytes_free.write_to_mem(bytes_free, mem)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocErr {
    pub kind: AllocErrKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocErrKind {
    Oom,
}

impl From<AllocErr> for Exception {
    fn from(err: AllocErr) -> Self {
        Self::AllocErr { err }
    }
}

pub fn init(
    strat: Strategy,
    mut region: TaggedCapability,
    mem: &mut Memory,
) -> Result<TaggedCapability, Exception> {
    /* TODOO: this must invalidate all capabilities matching 'region' before
     * returning to prevent caller from saving the capability and using it to
     * mess with the allocator */
    region = region.set_addr(region.start());
    let mut ret = region;
    mem.write(region, strat)?;
    region = region.set_addr(region.addr().add(Strategy::LAYOUT.size));
    let ator_cap = match strat {
        Strategy::Bump => {
            region = region.set_addr(region.addr().align_to(BumpAlloc::LAYOUT.align));
            let ator_cap =
                region.set_bounds(region.addr(), region.addr().add(BumpAlloc::LAYOUT.size));
            region = region.set_bounds(ator_cap.endb(), region.endb());
            let ator = BumpAlloc::new(region);
            mem.write(ator_cap, ator)?;
            ator_cap
        }
    };
    ret = ret.set_bounds(ret.start(), ator_cap.endb());
    Ok(ret)
}

pub fn deinit(ator: TaggedCapability, mem: &mut Memory) -> Result<TaggedCapability, Exception> {
    todo!()
}

pub fn alloc(
    mut ator: TaggedCapability,
    layout: Layout,
    mem: &mut Memory,
) -> Result<TaggedCapability, Exception> {
    /* TODO: very bad things can happen if ator has been mutated since being
     * returned by super::new. until capabilities can be sealed (and the
     * immutability of ator is guaranteed), this function is optimistic and
     * undermines everything :) */
    let strat: Strategy = mem.read(ator)?;
    ator = ator.set_addr(ator.addr().add(Strategy::LAYOUT.size));
    let ation = match strat {
        Strategy::Bump => {
            ator = ator.set_addr(ator.addr().align_to(BumpAlloc::LAYOUT.align));
            let mut bump: BumpAlloc = mem.read(ator)?;
            let ation = bump.alloc(layout)?;
            mem.write(ator, bump)?;
            ation
        }
    };
    Ok(ation)
}

pub fn free(
    ator: TaggedCapability,
    ation: TaggedCapability,
    mem: &mut Memory,
) -> Result<(), Exception> {
    todo!()
}

pub fn stat(mut ator: TaggedCapability, mem: &Memory) -> Result<Stats, Exception> {
    // TODO: reading allocator is dup code
    let strat: Strategy = mem.read(ator)?;
    ator = ator.set_addr(ator.addr().add(Strategy::LAYOUT.size));
    let stat = match strat {
        Strategy::Bump => {
            ator = ator.set_addr(ator.addr().align_to(BumpAlloc::LAYOUT.align));
            let bump: BumpAlloc = mem.read(ator)?;
            bump.stat()
        }
    };
    Ok(stat)
}
