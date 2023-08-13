use bitvec::slice::BitSlice;

use core::slice;

use crate::abi::{self, FieldStep, Layout, Ty};
use crate::capability::Address;
use crate::exception::Exception;
use crate::int::UAddr;

#[derive(Debug)]
pub struct StructRef<'fields, 'src, 'valid> {
    logic: StructLogic<'fields>,
    src: &'src [u8],
    addr: Address,
    valid: &'valid BitSlice<u8>,
}

#[derive(Debug)]
pub struct StructMut<'fields, 'dst, 'valid> {
    logic: StructLogic<'fields>,
    dst: &'dst mut [u8],
    addr: Address,
    valid: &'valid mut BitSlice<u8>,
}

impl<'fields, 'src, 'valid> StructRef<'fields, 'src, 'valid> {
    pub fn new(
        src: &'src [u8],
        addr: Address,
        valid: &'valid BitSlice<u8>,
        fields: &'fields [Layout],
    ) -> Self {
        Self {
            logic: StructLogic::new(fields),
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
        let valid = &self.valid[abi::gran_span(self.addr, offset)..][..=abi::gran_span(addr, size)];

        T::read(src, addr, valid)
    }
}

impl<'fields, 'dst, 'valid> StructMut<'fields, 'dst, 'valid> {
    pub fn new(
        dst: &'dst mut [u8],
        addr: Address,
        valid: &'valid mut BitSlice<u8>,
        fields: &'fields [Layout],
    ) -> Self {
        Self {
            logic: StructLogic::new(fields),
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
        let valid =
            &mut self.valid[abi::gran_span(self.addr, offset)..][..=abi::gran_span(addr, size)];

        src.write(dst, addr, valid)
    }
}

#[derive(Debug)]
pub struct StructLogic<'fields> {
    fields: slice::Iter<'fields, Layout>,
    cur_offset: UAddr,
}

impl<'fields> StructLogic<'fields> {
    pub fn new(fields: &'fields [Layout]) -> Self {
        Self {
            fields: fields.iter(),
            cur_offset: 0,
        }
    }
}

impl Iterator for StructLogic<'_> {
    type Item = (
        Layout, // layout of field
        UAddr,  // field offset from start
    );

    fn next(&mut self) -> Option<Self::Item> {
        let field = *self.fields.next()?;
        let step = FieldStep::new(field, self.cur_offset);
        self.cur_offset = step.cur_offset;
        Some((field, step.field_offset))
    }
}
