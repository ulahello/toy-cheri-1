use core::mem;

pub type UGran = u64;
pub type SGran = i64;
pub type UAddr = u16;
pub type SAddr = i16;

pub const UADDR_SIZE: u8 = mem::size_of::<UAddr>() as _;
pub const UGRAN_SIZE: u8 = mem::size_of::<UGran>() as _;

pub const UNINIT: UAddr = UAddr::from_le_bytes([UNINIT_BYTE; UADDR_SIZE as _]);
pub const UNINIT_BYTE: u8 = 0x55;

pub const fn gran_sign(u: UGran) -> SGran {
    SGran::from_le_bytes(u.to_le_bytes())
}

pub const fn gran_unsign(s: SGran) -> UGran {
    UGran::from_le_bytes(s.to_le_bytes())
}

pub const fn addr_sign(u: UAddr) -> SAddr {
    SAddr::from_le_bytes(u.to_le_bytes())
}

pub const fn addr_unsign(s: SAddr) -> UAddr {
    UAddr::from_le_bytes(s.to_le_bytes())
}
