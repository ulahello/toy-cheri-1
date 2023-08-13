mod bump;

use bitflags::bitflags;
use bitvec::slice::BitSlice;

use crate::abi::{self, Align, CustomFields, Layout, StructMut, StructRef, Ty};
use crate::access::MemAccessKind;
use crate::capability::{Address, OType, Permissions, TaggedCapability};
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
        let mut fields = StructRef::new(src, addr, valid, Self::FIELDS);
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
        let mut fields = StructMut::new(dst, addr, valid, Self::FIELDS);
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
        let mut fields = StructRef::new(src, addr, valid, Self::FIELDS);
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
        let mut fields = StructMut::new(dst, addr, valid, Self::FIELDS);
        fields.write_next(self.strat)?;
        fields.write_next(self.flags)?;
        Ok(())
    }
}

fn magic_seal(cap: TaggedCapability) -> TaggedCapability {
    cap.seal(cap.set_perms(cap.perms() | Permissions::SEAL))
}

fn magic_unseal(cap: TaggedCapability) -> TaggedCapability {
    cap.unseal(cap.set_perms(cap.perms() | Permissions::UNSEAL))
}

/// Initialize an allocator.
///
/// Pass ownership of `region` to a new allocator with the specified
/// configuration. On success, returns a sealed capability that can be later
/// unsealed by the allocator to access the region.
pub fn init(
    strat: Strategy,
    flags: InitFlags,
    region: TaggedCapability,
    mem: &mut Memory,
) -> Result<TaggedCapability, Exception> {
    // TODO: if sealed, set_addr invalidates region which leads to confusing error message
    let region = region.set_addr(region.start()); // reset address to region start
    region.check_access(
        MemAccessKind::Write,
        OType::VALID_ALIGN,
        Some(region.span_len()),
    )?;

    /* NOTE: invalidate all capabilities matching 'region' before returning to
     * prevent caller from saving the capability and using it to mess with the
     * allocator */
    revoke::by_bounds(mem, region.start(), region.endb())?;

    let mut fields = CustomFields::new(region);
    let header = Header { strat, flags };
    fields.write_next(header, mem)?;
    match strat {
        Strategy::Bump => {
            let ator_cap = fields.peek::<BumpAlloc>();
            let ator = BumpAlloc::new(region.set_bounds(ator_cap.endb(), region.endb()));
            fields.write_next(ator, mem)?;
        }
    }
    /* NOTE: the sealing capability is not constructable by userspace because we
     * revoke capabilities pointing into the region */
    let region = magic_seal(region);
    Ok(region)
}

/// De-initializes an allocator.
///
/// This frees all memory allocated by the allocator and returns the original
/// span passed to [`init`].
pub fn deinit(ator: TaggedCapability, mem: &mut Memory) -> Result<TaggedCapability, Exception> {
    let ator = magic_unseal(ator);
    let mut fields = CustomFields::new(ator);
    let header: Header = fields.read_next(mem)?;
    match header.strat {
        Strategy::Bump => {
            let bump_cap = fields.peek::<BumpAlloc>();
            let mut bump: BumpAlloc = fields.read_next(mem)?;
            bump.free_all();
            mem.write(bump_cap, bump)?;
        }
    }
    if header.flags.contains(InitFlags::INIT_ON_FREE) {
        mem.memset(ator, ator.span_len(), UNINIT_BYTE)?;
    }
    revoke::by_bounds(mem, ator.start(), ator.endb())?;
    Ok(ator)
}

pub fn alloc(
    ator: TaggedCapability,
    layout: Layout,
    mem: &mut Memory,
) -> Result<TaggedCapability, Exception> {
    let ator = magic_unseal(ator);
    let mut fields = CustomFields::new(ator);
    let header: Header = fields.read_next(mem)?;
    let ation = match header.strat {
        Strategy::Bump => {
            let bump_cap = fields.peek::<BumpAlloc>();
            let mut bump: BumpAlloc = fields.read_next(mem)?;
            let ation = bump.alloc(header, layout)?;
            mem.write(bump_cap, bump)?;
            ation
        }
    };
    if header.flags.contains(InitFlags::INIT_ON_ALLOC) {
        mem.memset(ation, ation.span_len(), UNINIT_BYTE)?;
    }
    Ok(ation)
}

pub fn free(
    ator: TaggedCapability,
    _ation: TaggedCapability,
    _mem: &mut Memory,
) -> Result<(), Exception> {
    let _ator = magic_unseal(ator);
    todo!()
}

pub fn free_all(ator: TaggedCapability, mem: &mut Memory) -> Result<(), Exception> {
    let ator = magic_unseal(ator);
    let mut fields = CustomFields::new(ator);
    let header: Header = fields.read_next(mem)?;
    match header.strat {
        Strategy::Bump => {
            let bump_cap = fields.peek::<BumpAlloc>();
            let mut bump: BumpAlloc = fields.read_next(mem)?;
            bump.free_all();
            revoke::by_bounds(mem, bump.inner.start(), bump.inner.endb())?;
            if header.flags.contains(InitFlags::INIT_ON_FREE) {
                mem.memset(bump.inner, bump.inner.span_len(), UNINIT_BYTE)?;
            }
            mem.write(bump_cap, bump)?;
        }
    }
    Ok(())
}

pub fn stat(ator: TaggedCapability, mem: &Memory) -> Result<Stats, Exception> {
    let ator = magic_unseal(ator);
    let mut fields = CustomFields::new(ator);
    let header: Header = fields.read_next(mem)?;
    let stat = match header.strat {
        Strategy::Bump => {
            let bump: BumpAlloc = fields.read_next(mem)?;
            bump.stat(header)
        }
    };
    Ok(stat)
}
