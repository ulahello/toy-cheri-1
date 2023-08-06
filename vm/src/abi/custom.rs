use crate::abi::{FieldStep, Ty};
use crate::capability::TaggedCapability;
use crate::exception::Exception;
use crate::int::UAddr;
use crate::mem::Memory;

/* TODO: should this work for registers? */

#[derive(Debug)]
pub struct CustomFields {
    base: TaggedCapability,
    cur_offset: UAddr,
}

impl CustomFields {
    pub const fn new(base: TaggedCapability) -> Self {
        Self {
            base,
            cur_offset: 0,
        }
    }

    fn peek_inner<T: Ty>(&self) -> (TaggedCapability, UAddr) {
        let layout = T::LAYOUT;
        let step = FieldStep::new(layout, self.cur_offset);
        let mut cap = self.base;
        cap = cap.set_addr(cap.addr().add(step.field_offset));
        cap = cap.set_bounds(cap.addr(), cap.addr().add(layout.size));
        (cap, step.cur_offset)
    }

    pub fn peek<T: Ty>(&self) -> TaggedCapability {
        self.peek_inner::<T>().0
    }

    pub fn read_next<T: Ty>(&mut self, mem: &Memory) -> Result<T, Exception> {
        let (field, cur_offset) = self.peek_inner::<T>();
        let val = mem.read(field)?;
        self.cur_offset = cur_offset;
        Ok(val)
    }

    pub fn write_next<T: Ty>(&mut self, src: T, mem: &mut Memory) -> Result<(), Exception> {
        let (field, cur_offset) = self.peek_inner::<T>();
        mem.write(field, src)?;
        self.cur_offset = cur_offset;
        Ok(())
    }
}
