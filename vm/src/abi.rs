use core::fmt;

use crate::int::UAddr;

#[derive(Clone, Copy, Debug)]
pub struct Align(UAddr /* must be nonzero power of two */);

impl Align {
    pub const fn new(align: UAddr) -> Option<Self> {
        if align.is_power_of_two() {
            Some(Self(align))
        } else {
            None
        }
    }

    pub const fn get(self) -> UAddr {
        self.0
    }
}

impl fmt::Display for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}
