use bitflags::bitflags;
use bitvec::slice::BitSlice;

use core::cmp::Ordering;
use core::fmt::{self, Write};

use crate::abi::{Align, Layout, Ty};
use crate::access::{MemAccess, MemAccessKind};
use crate::exception::Exception;
use crate::int::{gran_sign, SAddr, UAddr, UGran, UGRAN_SIZE, UNINIT};

/* TODOOO: implement sealed capabilities using metadata */

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Address(pub UAddr);

impl Address {
    pub const BITS: u8 = 40;

    pub const fn add(self, offset: UAddr) -> Self {
        Self(self.0.wrapping_add(offset))
    }

    pub const fn sub(self, offset: UAddr) -> Self {
        Self(self.0.wrapping_sub(offset))
    }

    pub const fn offset(self, offset: SAddr) -> Self {
        Self(self.0.wrapping_add_signed(offset))
    }

    pub const fn get(self) -> UAddr {
        self.0 & (UAddr::MAX >> (UAddr::BITS - Self::BITS as u32))
    }

    pub const fn gran(self) -> Granule {
        Granule(self.get() / UGRAN_SIZE as UAddr)
    }

    pub const fn is_aligned_to(self, align: Align) -> bool {
        self.get() % align.get() == 0
    }

    pub const fn align_to(self, align: Align) -> Self {
        if self.is_aligned_to(align) {
            self
        } else {
            self.align_up(align)
        }
    }

    pub const fn align_up(self, align: Align) -> Self {
        Self(self.get().next_multiple_of(align.get()))
    }

    pub const fn align_down(self, align: Align) -> Self {
        // only keep bits align and up
        Self(self.get() & !(align.get() - 1))
    }
}

impl Ty for Address {
    // TODO: should these only occupy Self::BITS?
    const LAYOUT: Layout = UAddr::LAYOUT;

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        Ok(Self(UAddr::read(src, addr, valid)?))
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        self.get().write(dst, addr, valid)
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for Address {}

impl PartialOrd for Address {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Address {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get().cmp(&other.get())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pad = Self::BITS as usize / 4;
        write!(f, "0x{:0pad$x}", self.get())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Granule(pub UAddr);

impl Granule {
    pub const fn addr(self) -> Address {
        Address(self.0.checked_add(UGRAN_SIZE as UAddr).unwrap())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capability {
    addr: Address,
    start: Address,
    endb: Address,
    perms: Permissions,
}

impl Capability {
    pub const INVALID: Self = {
        const LITERALLY_ANY_ADDRESS: Address = Address(UNINIT);
        Self {
            addr: LITERALLY_ANY_ADDRESS,
            start: LITERALLY_ANY_ADDRESS,
            endb: LITERALLY_ANY_ADDRESS,
            perms: Permissions::empty(),
        }
    };

    pub const fn new(addr: Address, start: Address, endb: Address, perms: Permissions) -> Self {
        Self {
            addr,
            start,
            endb,
            perms,
        }
    }

    #[allow(clippy::erasing_op, clippy::identity_op)]
    pub const fn from_ugran(ugran: UGran) -> Self {
        Self {
            addr: Address(
                ((ugran >> (Address::BITS * 0))
                    & (UGran::MAX >> (UGran::BITS - Address::BITS as u32)))
                    as UAddr,
            ),
            start: Address(
                ((ugran >> (Address::BITS * 1))
                    & (UGran::MAX >> (UGran::BITS - Address::BITS as u32)))
                    as UAddr,
            ),
            endb: Address(
                ((ugran >> (Address::BITS * 2))
                    & (UGran::MAX >> (UGran::BITS - Address::BITS as u32)))
                    as UAddr,
            ),
            perms: Permissions::from_bits_truncate(
                ((ugran >> (Address::BITS * 3))
                    & (UGran::MAX >> (UGran::BITS - Permissions::BITS as u32)))
                    as _,
            ),
        }
    }

    #[allow(clippy::identity_op)]
    pub const fn to_ugran(self) -> UGran {
        self.addr.get() as UGran
            | (self.start.get() as UGran) << (Address::BITS * 1)
            | (self.endb.get() as UGran) << (Address::BITS * 2)
            | (self.perms.bits() as UGran) << (Address::BITS * 3)
    }

    pub const fn addr(self) -> Address {
        self.addr
    }

    #[must_use]
    pub const fn set_addr(mut self, new: Address) -> Self {
        self.addr = new;
        /* we don't check bounds and update `valid` here because that's expected
         * to be checked for every access. we only do that for changing the
         * bounds. */
        self
    }

    pub const fn start(self) -> Address {
        self.start
    }

    pub const fn endb(self) -> Address {
        self.endb
    }

    pub const fn perms(self) -> Permissions {
        self.perms
    }

    #[allow(clippy::len_without_is_empty)]
    pub const fn len(&self) -> UAddr {
        self.endb().get().saturating_sub(self.start().get())
    }
}

impl Ty for Capability {
    const LAYOUT: Layout = Layout {
        size: UGRAN_SIZE as _,
        align: Align::new(1).unwrap(),
    };

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        Ok(Self::from_ugran(UGran::read(src, addr, valid)?))
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        self.to_ugran().write(dst, addr, valid)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct TaggedCapability {
    capa: Capability,
    valid: bool,
}

impl TaggedCapability {
    pub const INVALID: Self = Self {
        capa: Capability::INVALID,
        valid: false,
    };

    // only for internal use!!!
    /* TODO: this should only be called once to bless the root capability, which
     * the initial program should use to construct capabilities as needed. since
     * the init program is running in userspace, there isn't any shenanigans of
     * creating a valid capability in rust land before it exists in the tag
     * controller. */
    pub const fn new(capability: Capability, valid: bool) -> Self {
        Self {
            capa: capability,
            valid,
        }
    }

    pub const fn from_ugran(ugran: UGran) -> Self {
        Self::new(Capability::from_ugran(ugran), false)
    }

    pub const fn to_ugran(self) -> UGran {
        self.capa.to_ugran()
    }

    pub const fn capability(self) -> Capability {
        self.capa
    }

    pub const fn is_valid(&self) -> bool {
        self.valid
    }

    pub const fn addr(self) -> Address {
        self.capa.addr()
    }

    pub const fn set_addr(self, new: Address) -> Self {
        Self {
            capa: self.capa.set_addr(new),
            valid: self.valid,
        }
    }

    pub const fn start(self) -> Address {
        self.capa.start()
    }

    pub const fn endb(self) -> Address {
        self.capa.endb()
    }

    pub const fn set_bounds(self, start: Address, endb: Address) -> Self {
        /* TODO: is it okay if `endb < start`? should we invalidate it now or
         * wait for access to raise exception? */
        // HACK: address should be const comparable
        let valid = start.get() >= self.capa.start.get() && endb.get() <= self.capa.endb.get();
        Self {
            capa: Capability {
                addr: self.capa.addr,
                start,
                endb,
                perms: self.capa.perms,
            },
            valid,
        }
    }

    pub const fn perms(self) -> Permissions {
        self.capa.perms()
    }

    pub const fn set_perms(self, perms: Permissions) -> Self {
        // new perms are valid if they at most disable a permission.
        let valid_is_valid = (!perms.x() || self.capa.perms.x())
            && (!perms.w() || self.capa.perms.w())
            && (!perms.r() || self.capa.perms.r());
        Self {
            capa: Capability {
                addr: self.capa.addr,
                start: self.capa.start,
                endb: self.capa.endb,
                perms,
            },
            valid: self.is_valid() && valid_is_valid,
        }
    }

    pub const fn set_perms_from(self, perms: Permissions, mut root: Self) -> Self {
        root = root
            .set_addr(self.addr())
            .set_bounds(self.start(), self.endb());
        // TODO: if root has tighter perms, thisll fail even if self may have had enough. document this behavior?
        root = root.set_perms(perms);
        root
    }

    pub const fn access(&self, kind: MemAccessKind, align: Align, len: Option<UAddr>) -> MemAccess {
        MemAccess {
            tcap: *self,
            len,
            align,
            kind,
        }
    }

    pub const fn check_given_access(&self, access: MemAccess) -> Result<(), Exception> {
        if self.is_valid() && access.is_bounded() && access.perms_grant() && access.is_aligned() {
            Ok(())
        } else {
            Err(Exception::InvalidMemAccess { access })
        }
    }

    pub const fn check_access(
        &self,
        kind: MemAccessKind,
        align: Align,
        len: Option<UAddr>,
    ) -> Result<(), Exception> {
        self.check_given_access(self.access(kind, align, len))
    }
}

impl Ty for TaggedCapability {
    const LAYOUT: Layout = Layout {
        size: Capability::LAYOUT.size,
        align: Align::new(UGRAN_SIZE as _).unwrap(),
    };

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        debug_assert_eq!(valid.len(), 1);
        let capa = Capability::read(src, addr, valid)?;
        let valid = valid[0];
        Ok(Self::new(capa, valid))
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        debug_assert_eq!(valid.len(), 1);
        self.capa.write(dst, addr, valid)?;
        *valid.get_mut(0).unwrap() = self.is_valid();
        Ok(())
    }
}

impl fmt::Debug for TaggedCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.valid {
            f.debug_struct("TaggedCapability")
                .field("addr", &self.addr())
                .field("start", &self.start())
                .field("endb", &self.endb())
                .field("perms", &self.perms())
                .finish()
        } else {
            let u = self.to_ugran();
            let s = gran_sign(u);
            if s.is_negative() {
                write!(f, "{{u{u} s{s}}}")
            } else {
                write!(f, "{u}")
            }
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Permissions: u8 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
        const EXEC = 0b00000100;
    }
}

impl Permissions {
    pub const BITS: u8 = 3;

    pub const fn r(self) -> bool {
        self.contains(Self::READ)
    }

    pub const fn w(self) -> bool {
        self.contains(Self::WRITE)
    }

    pub const fn x(self) -> bool {
        self.contains(Self::EXEC)
    }

    pub const fn grants_access(&self, kind: MemAccessKind) -> bool {
        match kind {
            MemAccessKind::Read => self.r(),
            MemAccessKind::Write => self.w(),
            MemAccessKind::Execute => self.x(),
        }
    }
}

impl Ty for Permissions {
    const LAYOUT: Layout = u8::LAYOUT;

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

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const NOPE: char = '-';
        f.write_char(if self.r() { 'r' } else { NOPE })?;
        f.write_char(if self.w() { 'w' } else { NOPE })?;
        f.write_char(if self.x() { 'x' } else { NOPE })?;
        Ok(())
    }
}

impl fmt::Debug for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
