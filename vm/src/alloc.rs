use crate::int::UAddr;
use crate::mem::Memory;

/* TODO: temporal safety (this will do that allegedly) */
/* TODO: concept of nested allocators with different allocation strategies.

for example we may allocate some length of memory to be a program's stack. this
span is itself an allocator using a stack allocation strategy.

pub enum Strategy {
    LinkedList,
    Stack,
}
 */

pub struct Allocator {}

// TODO: store args & return capabilities in Z0 & Z1 so theyre correctly tagged
impl Allocator {
    pub const fn new(mem: &Memory) -> Self {
        todo!()
    }

    pub fn alloc(mem: &mut Memory, bytes: UAddr) {
        todo!()
    }
}
