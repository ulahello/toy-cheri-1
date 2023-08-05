mod bump;

use bitflags::bitflags;
use bitvec::slice::BitSlice;

use crate::abi::{self, Align, FieldsMut, FieldsRef, Layout, Ty};
use crate::capability::{Address, TaggedCapability};
use crate::exception::Exception;
use crate::int::{UAddr, UNINIT_BYTE};
use crate::mem::Memory;
use crate::revoke;

use bump::BumpAlloc;

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

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        Self::from_byte(u8::read(src, addr, valid)?)
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        self.to_byte().write(dst, addr, valid)
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

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        Ok(Self::from_bits_truncate(u8::read(src, addr, valid)?))
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        let repr: u8 = self.bits();
        repr.write(dst, addr, valid)
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
    const LAYOUT: Layout = abi::layout(Self::FIELDS);

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        let mut fields = FieldsRef::new(src, addr, valid, Self::FIELDS);
        Ok(Self {
            strategy: fields.read_next::<Strategy>()?,
            flags: fields.read_next::<InitFlags>()?,
            bytes_free: fields.read_next::<UAddr>()?,
        })
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        let mut fields = FieldsMut::new(dst, addr, valid, Self::FIELDS);
        fields.write_next(self.strategy)?;
        fields.write_next(self.flags)?;
        fields.write_next(self.bytes_free)?;
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
    const LAYOUT: Layout = abi::layout(Self::FIELDS);

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        let mut fields = FieldsRef::new(src, addr, valid, Self::FIELDS);
        Ok(Self {
            strat: fields.read_next::<Strategy>()?,
            flags: fields.read_next::<InitFlags>()?,
        })
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        let mut fields = FieldsMut::new(dst, addr, valid, Self::FIELDS);
        fields.write_next(self.strat)?;
        fields.write_next(self.flags)?;
        Ok(())
    }
}

/// Initialize an allocator.
///
/// Pass ownership of `region` to a new allocator with the specified
/// configuration. On success, returns a sealed capability that can be later
/// unsealed by the allocator to access the region.
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

/// De-initializes an allocator.
///
/// This frees all memory allocated by the allocator and returns the original
/// span passed to [`init`].
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
