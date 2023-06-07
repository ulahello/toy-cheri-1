use core::fmt;
use std::fmt::Write;

use crate::abi::Align;
use crate::access::{MemAccess, MemAccessKind};
use crate::exception::Exception;
use crate::int::{UAddr, UGran, UGRAN_SIZE};

/* TODO: implement sealed capabilities using metadata */

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

    pub const fn get(self) -> UAddr {
        self.0 & (UAddr::MAX >> (UAddr::BITS - Self::BITS as u32))
    }

    pub const fn gran(self) -> Granule {
        Granule(self.get() / UGRAN_SIZE as UAddr)
    }

    pub const fn is_aligned_to(self, align: Align) -> bool {
        self.get() % align.get() == 0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pad = Self::BITS as usize / 4;
        write!(f, "0x{:0pad$x}", self.get())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self}")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Granule(pub UAddr);

impl Granule {
    pub const fn addr(self) -> Address {
        // TODO: overflow
        Address(self.0 * UGRAN_SIZE as UAddr)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capability {
    pub(crate) addr: Address,
    start: Address,
    endb: Address,
    perms: Permissions,
}

impl Capability {
    pub const SIZE: u8 = UGRAN_SIZE;

    pub const INVALID: Self = {
        const LITERALLY_ANY_ADDRESS: Address = Address(0);
        Self {
            addr: LITERALLY_ANY_ADDRESS,
            start: LITERALLY_ANY_ADDRESS,
            endb: LITERALLY_ANY_ADDRESS,
            perms: Permissions {
                r: false,
                w: false,
                x: false,
            },
        }
    };

    pub const ALIGN: Align = Align::new(UGRAN_SIZE as _).unwrap();

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
            perms: Permissions::from_data(
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
            | (self.perms.to_data() as UGran) << (Address::BITS * 3)
    }

    pub const fn is_bounded(&self) -> bool {
        self.is_addr_bounded(self.addr)
    }

    pub const fn is_addr_bounded(&self, addr: Address) -> bool {
        // HACK: address should be const comparable
        addr.get() >= self.start.get() && addr.get() < self.endb.get()
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
}

/* TODO: this essentially describes a usize index into tag controller. it might
 * be dangerous to use this in api because the thing that this represents could
 * change behind its back. its like a reference but cached. */
#[derive(Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct TaggedCapability {
    pub(crate) capa: Capability,
    pub(crate) valid: bool,
}

impl TaggedCapability {
    pub const SIZE: u8 = UGRAN_SIZE;

    pub const INVALID: Self = Self {
        capa: Capability::INVALID,
        valid: false,
    };

    // only for internal use!!!
    pub const fn new(capability: Capability, valid: bool) -> Self {
        Self {
            capa: capability,
            valid,
        }
    }

    pub const fn capability(self) -> Capability {
        self.capa
    }

    pub const fn is_bounded(&self) -> bool {
        self.capa.is_bounded()
    }

    pub const fn is_addr_bounded(&self, addr: Address) -> bool {
        self.capa.is_addr_bounded(addr)
    }

    pub const fn is_valid(&self) -> bool {
        self.valid
    }

    pub const fn addr(self) -> Address {
        self.capa.addr()
    }

    #[must_use]
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

    #[must_use]
    pub const fn set_bounds(self, start: Address, endb: Address) -> Self {
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

    #[must_use]
    pub const fn set_perms(self, perms: Permissions) -> Self {
        // new perms are valid if they at most disable a permission.
        let valid_is_valid = (!perms.x || self.capa.perms.x)
            && (!perms.w || self.capa.perms.w)
            && (!perms.r || self.capa.perms.r);
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

    #[must_use]
    pub const fn set_perms_from(self, perms: Permissions, mut root: Self) -> Self {
        root = root.set_bounds(self.start(), self.endb());
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

impl fmt::Debug for TaggedCapability {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.valid {
            f.debug_struct("TaggedCapability")
                .field("addr", &self.addr())
                .field("start", &self.start())
                .field("endb", &self.endb())
                .field("perms", &self.perms())
                .finish()
        } else {
            write!(f, "{}", self.capability().to_ugran())
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    pub r: bool,
    pub w: bool,
    pub x: bool,
}

impl Permissions {
    pub const BITS: u8 = 3;

    pub const fn grants_access(&self, kind: MemAccessKind) -> bool {
        match kind {
            MemAccessKind::Read => self.r,
            MemAccessKind::Write => self.w,
            MemAccessKind::Execute => self.x,
        }
    }

    pub const fn from_data(data: u8) -> Self {
        Self {
            r: (data & (1 << 0)) != 0,
            w: (data & (1 << 1)) != 0,
            x: (data & (1 << 2)) != 0,
        }
    }

    pub const fn to_data(self) -> u8 {
        (self.r as u8) | ((self.w as u8) << 1) | ((self.x as u8) << 2)
    }
}

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const NOPE: char = '-';
        f.write_char(if self.r { 'r' } else { NOPE })?;
        f.write_char(if self.w { 'w' } else { NOPE })?;
        f.write_char(if self.x { 'x' } else { NOPE })?;
        Ok(())
    }
}

impl fmt::Debug for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self}")
    }
}
