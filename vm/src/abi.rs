mod core_impls;

use bitvec::slice::BitSlice;

use core::{fmt, slice};

use crate::capability::Address;
use crate::exception::Exception;
use crate::int::UAddr;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Align(u8);

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
        Ok(Self(repr))
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
        let mut fields = FieldsRef::new(src, addr, valid, Self::FIELDS);
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
        let mut fields = FieldsMut::new(dst, addr, valid, Self::FIELDS);
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

    /// Read byte representation of `self` from `src`. This read may observe the
    /// tag controller. For such circumstances, the bit slice `valid` is
    /// provided. `addr` is equivalent to the starting address of `dst`.
    ///
    /// # Notes
    ///
    /// - The length of `dst` must equal `Self::LAYOUT.size`.
    /// - The length of `valid` must equal the number of granules which `dst`
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
        offset = FieldsLogic::field_step(field, offset).cur_offset;

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

pub struct FieldsRef<'fields, 'src, 'valid> {
    logic: FieldsLogic<'fields>,
    src: &'src [u8],
    addr: Address,
    valid: &'valid BitSlice<u8>,
}

pub struct FieldsMut<'fields, 'dst, 'valid> {
    logic: FieldsLogic<'fields>,
    dst: &'dst mut [u8],
    addr: Address,
    valid: &'valid mut BitSlice<u8>,
}

impl<'fields, 'src, 'valid> FieldsRef<'fields, 'src, 'valid> {
    pub fn new(
        src: &'src [u8],
        addr: Address,
        valid: &'valid BitSlice<u8>,
        fields: &'fields [Layout],
    ) -> Self {
        Self {
            logic: FieldsLogic::new(fields),
            src,
            addr,
            valid,
        }
    }

    pub fn read_next<T: Ty>(&mut self) -> Result<T, Exception> {
        let (layout, offset) = self.logic.next().unwrap();
        debug_assert_eq!(T::LAYOUT, layout);
        let size = layout.size;

        let src = &self.src[offset as usize..][..size as usize];
        let addr = self.addr.add(offset);
        let valid = &self.valid[FieldsLogic::gran_span(self.addr, offset)..]
            [..=FieldsLogic::gran_span(addr, size)];

        T::read(src, addr, valid)
    }
}

impl<'fields, 'dst, 'valid> FieldsMut<'fields, 'dst, 'valid> {
    pub fn new(
        dst: &'dst mut [u8],
        addr: Address,
        valid: &'valid mut BitSlice<u8>,
        fields: &'fields [Layout],
    ) -> Self {
        Self {
            logic: FieldsLogic::new(fields),
            dst,
            addr,
            valid,
        }
    }

    pub fn write_next<T: Ty>(&mut self, src: T) -> Result<(), Exception> {
        let (layout, offset) = self.logic.next().unwrap();
        debug_assert_eq!(T::LAYOUT, layout);
        let size = layout.size;

        let dst = &mut self.dst[offset as usize..][..size as usize];
        let addr = self.addr.add(offset);
        let valid = &mut self.valid[FieldsLogic::gran_span(self.addr, offset)..]
            [..=FieldsLogic::gran_span(addr, size)];

        src.write(dst, addr, valid)
    }
}

pub(crate) struct FieldsLogic<'a> {
    fields: slice::Iter<'a, Layout>,
    cur_offset: UAddr,
}

impl<'a> FieldsLogic<'a> {
    pub fn new(fields: &'a [Layout]) -> Self {
        Self {
            fields: fields.iter(),
            cur_offset: 0,
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
}

struct FieldStep {
    pub field_offset: UAddr,
    pub cur_offset: UAddr,
}

impl FieldsLogic<'_> {
    const fn field_step(field: Layout, mut cur_offset: UAddr) -> FieldStep {
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

impl Iterator for FieldsLogic<'_> {
    type Item = (
        Layout, // layout of field
        UAddr,  // field offset from start
    );

    fn next(&mut self) -> Option<Self::Item> {
        let field = *self.fields.next()?;
        let step = FieldsLogic::field_step(field, self.cur_offset);
        self.cur_offset = step.cur_offset;
        Some((field, step.field_offset))
    }
}

impl Drop for FieldsLogic<'_> {
    fn drop(&mut self) {
        debug_assert_eq!(None, self.fields.next());
    }
}
