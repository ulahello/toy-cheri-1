use core::fmt;

use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::int::UAddr;
use crate::mem::Memory;

// TODO: continue to reduce boilerplate implementing Ty

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Align(UAddr /* must be nonzero power of two */);

impl Align {
    pub const MIN: Self = Self::new(1).unwrap();

    pub const fn new(align: UAddr) -> Option<Self> {
        // power of two implies nonzero
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

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub size: UAddr,
    pub align: Align,
}

impl fmt::Display for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

// NOTE: prefer methods on Memory because it explicitly checks accesses
pub trait Ty: Copy + Sized + fmt::Debug {
    const LAYOUT: Layout;

    fn read_from_mem(src: TaggedCapability, mem: &Memory) -> Result<Self, Exception>;

    fn write_to_mem(&self, dst: TaggedCapability, mem: &mut Memory) -> Result<(), Exception>;
}

pub struct Fields<'a> {
    fields: core::slice::Iter<'a, Layout>,
    start: TaggedCapability,
    cur_offset: UAddr,
}

impl<'a> Fields<'a> {
    pub fn new(start: TaggedCapability, fields: &'a [Layout]) -> Self {
        Self {
            fields: fields.iter(),
            start,
            cur_offset: 0,
        }
    }

    pub const fn layout(fields: &[Layout]) -> Layout {
        let mut align = Align::MIN;
        let mut offset: UAddr = 0;

        let mut idx = 0;
        while idx < fields.len() {
            let field = fields[idx];

            // TODO: unoptimized
            // bump to aligned start of field
            while offset % field.align.get() != 0 {
                // 2.next_multiple_of_two() == 2, so add 1 to always go up
                offset = (offset + 1).next_power_of_two();
            }
            // bamf out
            offset += field.size;

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
}

impl<'a> Iterator for Fields<'a> {
    type Item = TaggedCapability;

    fn next(&mut self) -> Option<Self::Item> {
        let field = self.fields.next()?;

        // TODO: dup code

        while self.cur_offset % field.align.get() != 0 {
            // 2.next_multiple_of_two() == 2, so add 1 to always go up
            self.cur_offset = (self.cur_offset + 1).next_power_of_two();
        }

        let field_offset = self.cur_offset;

        // bamf out
        self.cur_offset += field.size;

        let mut field_tcap = self.start.set_addr(self.start.addr().add(field_offset));
        // tighten capability bounds to only the field data (not padding)
        field_tcap = field_tcap.set_bounds(field_tcap.addr(), field_tcap.addr().add(field.size));

        Some(field_tcap)
    }
}
