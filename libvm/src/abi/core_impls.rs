use bitvec::slice::BitSlice;

use core::mem;

use super::{Align, Layout, Ty};
use crate::capability::Address;
use crate::exception::Exception;
use crate::int::{SAddr, SGran, UAddr, UGran};

/* boolean */

impl Ty for bool {
    const LAYOUT: Layout = u8::LAYOUT;

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        let repr = u8::read(src, addr, valid)?;
        Ok(repr != 0)
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        let repr: u8 = if self { 1 } else { 0 };
        repr.write(dst, addr, valid)
    }
}

/* integers */

macro_rules! int_impl {
    ($type:ty) => {
        impl Ty for $type {
            const LAYOUT: Layout = Layout {
                size: mem::size_of::<Self>() as _,
                align: Align::new(1).unwrap(),
            };

            fn read(src: &[u8], _addr: Address, _valid: &BitSlice<u8>) -> Result<Self, Exception> {
                debug_assert_eq!(usize::try_from(Self::LAYOUT.size), Ok(src.len()));
                Ok(Self::from_le_bytes(
                    src.try_into()
                        .expect("src.len() must equal Self::LAYOUT.size"),
                ))
            }

            fn write(
                self,
                dst: &mut [u8],
                _addr: Address,
                valid: &mut BitSlice<u8>,
            ) -> Result<(), Exception> {
                debug_assert_eq!(usize::try_from(Self::LAYOUT.size), Ok(dst.len()));

                // we are writing data, not a capability
                valid.fill(false);

                let repr = self.to_le_bytes();
                dst.copy_from_slice(&repr);

                Ok(())
            }
        }
    };
}

int_impl!(UGran);
int_impl!(UAddr);
int_impl!(u8);
int_impl!(u32);

int_impl!(SGran);
int_impl!(SAddr);
int_impl!(i8);
int_impl!(i32);
