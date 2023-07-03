mod bump;

use bitflags::bitflags;

use crate::abi::{Align, Fields, Layout, Ty};
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::int::{UAddr, UNINIT_BYTE};
use crate::mem::Memory;
use crate::revoke;

use bump::BumpAlloc;

/* TODOO: temporal safety (this will do that allegedly)
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

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct InitFlags: u8 {
        const INIT_ON_ALLOC = 0b00000001;
        const INIT_ON_FREE = 0b00000010;
    }
}

impl Ty for InitFlags {
    const LAYOUT: Layout = Layout {
        size: 1,
        align: Align::new(1).unwrap(),
    };

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let bits: u8 = mem.read(src)?;
        let flags =
            Self::from_bits(bits).ok_or(Exception::InvalidAllocInitFlags { flags: bits })?;
        Ok(flags)
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        mem.write::<u8>(dst, self.bits())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub strategy: Strategy,
    pub flags: InitFlags,
    pub bytes_free: UAddr,
}

impl Stats {
    const FIELDS: &'static [Layout] = &[Strategy::LAYOUT, InitFlags::LAYOUT, UAddr::LAYOUT];
}

impl Ty for Stats {
    const LAYOUT: Layout = Fields::layout(Self::FIELDS);

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let mut fields = Fields::new(src, Self::FIELDS);
        let strategy = fields.next().unwrap();
        let flags = fields.next().unwrap();
        let bytes_free = fields.next().unwrap();
        Ok(Self {
            strategy: Strategy::read_from_mem(strategy, mem)?,
            flags: InitFlags::read_from_mem(flags, mem)?,
            bytes_free: UAddr::read_from_mem(bytes_free, mem)?,
        })
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        let mut fields = Fields::new(dst, Self::FIELDS);
        let strategy = fields.next().unwrap();
        let flags = fields.next().unwrap();
        let bytes_free = fields.next().unwrap();
        self.strategy.write_to_mem(strategy, mem)?;
        self.flags.write_to_mem(flags, mem)?;
        self.bytes_free.write_to_mem(bytes_free, mem)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocErr {
    pub stats: Stats,
    pub requested: Layout,
    pub kind: AllocErrKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocErrKind {
    /// The requested layout cannot be allocated because there aren't enough bytes free.
    NotEnoughMem,

    /// The allocator reports 0 bytes free.
    Oom,
}

impl From<AllocErr> for Exception {
    fn from(err: AllocErr) -> Self {
        Self::AllocErr { err }
    }
}

#[derive(Clone, Copy, Debug)]
struct Header {
    strat: Strategy,
    flags: InitFlags,
}

impl Header {
    const FIELDS: &'static [Layout] = &[Strategy::LAYOUT, InitFlags::LAYOUT];
}

impl Ty for Header {
    const LAYOUT: Layout = Fields::layout(Self::FIELDS);

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception> {
        let mut fields = Fields::new(src, Self::FIELDS);
        let strat = fields.next().unwrap();
        let flags = fields.next().unwrap();
        Ok(Self {
            strat: Strategy::read_from_mem(strat, mem)?,
            flags: InitFlags::read_from_mem(flags, mem)?,
        })
    }

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
        let mut fields = Fields::new(dst, Self::FIELDS);
        let strat = fields.next().unwrap();
        let flags = fields.next().unwrap();
        self.strat.write_to_mem(strat, mem)?;
        self.flags.write_to_mem(flags, mem)?;
        Ok(())
    }
}

pub fn init(
    strat: Strategy,
    flags: InitFlags,
    mut region: TaggedCapability,
    mem: &mut Memory,
) -> Result<TaggedCapability, Exception> {
    /* NOTE: invalidate all capabilities matching 'region' before returning to
     * prevent caller from saving the capability and using it to mess with the
     * allocator */
    revoke::by_bounds(mem, region.start(), region.endb())?;

    let header = Header { strat, flags };
    region = region.set_addr(region.start());
    let mut ret = region;
    mem.write(region, header)?;
    region = region.set_addr(region.addr().add(Header::LAYOUT.size));
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
    free_all(ator, mem)?;
    todo!()
}

pub fn alloc(
    mut ator: TaggedCapability,
    layout: Layout,
    mem: &mut Memory,
) -> Result<TaggedCapability, Exception> {
    /* TODOO: very bad things can happen if ator has been mutated (or even used)
     * since being returned by super::init. until capabilities can be sealed
     * (and the immutability of ator is guaranteed), this function is optimistic
     * and undermines everything :) */
    let header: Header = mem.read(ator)?;
    ator = ator.set_addr(ator.addr().add(Header::LAYOUT.size));
    let ation = match header.strat {
        Strategy::Bump => {
            ator = ator.set_addr(ator.addr().align_to(BumpAlloc::LAYOUT.align));
            let mut bump: BumpAlloc = mem.read(ator)?;
            let ation = bump.alloc(header, layout)?;
            mem.write(ator, bump)?;
            ation
        }
    };
    if header.flags.contains(InitFlags::INIT_ON_ALLOC) {
        mem.memset(ation, ation.capability().len(), UNINIT_BYTE)?;
    }
    Ok(ation)
}

pub fn free(
    _ator: TaggedCapability,
    _ation: TaggedCapability,
    _mem: &mut Memory,
) -> Result<(), Exception> {
    todo!()
}

pub fn free_all(mut ator: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
    // TODO: reading allocator is dup code
    let header: Header = mem.read(ator)?;
    ator = ator.set_addr(ator.addr().add(Header::LAYOUT.size));
    match header.strat {
        Strategy::Bump => {
            ator = ator.set_addr(ator.addr().align_to(BumpAlloc::LAYOUT.align));
            let mut bump: BumpAlloc = mem.read(ator)?;
            bump.free_all();
            revoke::by_bounds(mem, bump.inner.start(), bump.inner.endb())?;
            if header.flags.contains(InitFlags::INIT_ON_FREE) {
                mem.memset(bump.inner, bump.inner.capability().len(), UNINIT_BYTE)?;
            }
            mem.write(ator, bump)?;
        }
    }
    Ok(())
}

pub fn stat(mut ator: TaggedCapability, mem: &Memory) -> Result<Stats, Exception> {
    // TODO: reading allocator is dup code
    let header: Header = mem.read(ator)?;
    ator = ator.set_addr(ator.addr().add(Header::LAYOUT.size));
    let stat = match header.strat {
        Strategy::Bump => {
            ator = ator.set_addr(ator.addr().align_to(BumpAlloc::LAYOUT.align));
            let bump: BumpAlloc = mem.read(ator)?;
            bump.stat(header)
        }
    };
    Ok(stat)
}
