use crate::capability::{Address, Capability};
use crate::exception::Exception;
use crate::mem::{Memory, TagController};

pub fn by_bounds(mem: &mut Memory, start: Address, endb: Address) -> Result<(), Exception> {
    /* iterate through every bit in the tag controller. if it's valid, check if
     * it matches the pattern. if it does, invalidate it. */
    let root = mem.root;
    for idx in 0..mem.tags.mem.len() {
        if mem.tags.mem[idx] {
            let cap: Capability = {
                let mut cap = None;
                if let Ok(reg) = u8::try_from(idx) {
                    if let Ok(gran) = mem.regs.read_untagged(reg) {
                        cap = Some(Capability::from_ugran(gran));
                    }
                }
                if let Some(cap) = cap {
                    cap
                } else {
                    let gran = TagController::idx_to_gran(idx)
                        .expect("enumerated tag controller index is valid");
                    /* NOTE: it's okay to create magic tcap here because
                     * revoking capabilities is conceptually a priveleged
                     * process */
                    mem.read(root.set_addr(gran.addr()))?
                }
            };
            if (cap.start() >= start && cap.start() <= endb)
                || (cap.endb() >= start && cap.endb() <= endb)
            {
                // this capability would have been able to access the pattern
                *mem.tags.mem.get_mut(idx).unwrap() = false;
            }
        }
    }
    Ok(())
}
