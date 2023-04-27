use core::mem;

pub type UGran = u128;
pub type UAddr = u64;

pub const UADDR_SIZE: u8 = mem::size_of::<UAddr>() as _;
pub const UGRAN_SIZE: u8 = mem::size_of::<UGran>() as _;

pub const UNINIT: UAddr = UAddr::from_le_bytes([0x55; UADDR_SIZE as _]);
