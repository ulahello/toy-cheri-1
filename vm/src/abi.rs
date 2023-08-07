mod core_impls;
mod custom;
mod structs;

use bitvec::slice::BitSlice;

use core::fmt;

use crate::capability::Address;
use crate::exception::Exception;
use crate::int::UAddr;

pub use custom::CustomFields;
pub use structs::{StructMut, StructRef};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Align(u8 /* must be less than UAddr::BITS */);

impl Align {
    pub const MIN: Self = Self::new(1).unwrap();

    pub const fn new(align: UAddr) -> Option<Self> {
        // power of two implies nonzero
        if align.is_power_of_two() {
            debug_assert!(align != 0);
            Some(Self(align.ilog2() as _))
        } else {
            None
        }
    }

    pub const fn get(self) -> UAddr {
        (2 as UAddr).pow(self.0 as _)
    }
}

impl Ty for Align {
    const LAYOUT: Layout = u8::LAYOUT;

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        let repr = u8::read(src, addr, valid)?;
        let trunc = repr & 0b0011_1111; // truncate to u6
        Ok(Self(trunc))
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        let repr: u8 = self.0;
        repr.write(dst, addr, valid)
    }
}

impl fmt::Debug for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Align").field(&self.get()).finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub size: UAddr,
    pub align: Align,
}

impl Ty for Layout {
    const LAYOUT: Layout = layout(Self::FIELDS);

    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception> {
        let mut fields = StructRef::new(src, addr, valid, Self::FIELDS);
        Ok(Self {
            size: fields.read_next::<UAddr>()?,
            align: fields.read_next::<Align>()?,
        })
    }

    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception> {
        let mut fields = StructMut::new(dst, addr, valid, Self::FIELDS);
        fields.write_next(self.size)?;
        fields.write_next(self.align)?;
        Ok(())
    }
}

impl Layout {
    const FIELDS: &'static [Layout] = &[UAddr::LAYOUT, Align::LAYOUT];
}

impl fmt::Display for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

pub trait Ty: Copy + Sized + fmt::Debug {
    const LAYOUT: Layout;

    /// Read byte representation of `Self` from `src`. This read may observe the
    /// tag controller. For such circumstances, the bit slice `valid` is
    /// provided. `addr` is equivalent to the starting address of `src`.
    ///
    /// # Notes
    ///
    /// - The length of `src` must equal `Self::LAYOUT.size`.
    /// - The length of `valid` must equal the number of granules which `src`
    /// spans.
    ///
    /// # Errors
    ///
    /// If `src` is not a valid instance of `Self`, or if the alignment of `src`
    /// is invalid, this must return an error.
    fn read(src: &[u8], addr: Address, valid: &BitSlice<u8>) -> Result<Self, Exception>;

    /// Write byte representation of `self` into `dst`. This write may have side
    /// effects on the tag controller. For such circumstances, the bit slice
    /// `valid` is provided. `addr` is equivalent to the starting address of
    /// `dst`.
    ///
    /// # Notes
    ///
    /// - The length of `dst` must equal `Self::LAYOUT.size`.
    /// - The length of `valid` must equal the number of granules which `dst`
    /// spans.
    ///
    /// # Errors
    ///
    /// If the alignment of `dst` is invalid, this must return an error.
    fn write(
        self,
        dst: &mut [u8],
        addr: Address,
        valid: &mut BitSlice<u8>,
    ) -> Result<(), Exception>;
}

pub const fn layout(fields: &[Layout]) -> Layout {
    let mut align = Align::MIN;
    let mut offset: UAddr = 0;

    let mut idx = 0;
    while idx < fields.len() {
        let field = fields[idx];
        offset = FieldStep::new(field, offset).cur_offset;

        // alignment of struct is max of all field alignments
        if field.align.get() > align.get() {
            align = field.align;
        }

        idx += 1;
    }

    Layout {
        size: offset,
        align,
    }
}

// TODO: overflow
/// Returns the index of the last granule in the given address span.
pub fn gran_span(addr: Address, size: UAddr) -> usize {
    if size == 0 {
        return 0;
    }
    let endb = addr.add(size);
    let end = endb.sub(1);
    let diff = end.gran().0 - addr.gran().0;
    usize::try_from(diff).unwrap()
}

struct FieldStep {
    pub field_offset: UAddr,
    pub cur_offset: UAddr,
}

impl FieldStep {
    const fn new(field: Layout, mut cur_offset: UAddr) -> FieldStep {
        // bump to aligned start of field
        while cur_offset % field.align.get() != 0 {
            // 2.next_multiple_of_two() == 2, so add 1 to always go up
            cur_offset = (cur_offset + 1).next_power_of_two();
        }
        let field_offset = cur_offset;
        cur_offset += field.size; // bamf out
        FieldStep {
            field_offset,
            cur_offset,
        }
    }
}
