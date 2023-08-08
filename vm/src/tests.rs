mod serde {
    use crate::capability::{Address, Capability, Permissions};
    use crate::int::UGran;
    use nanorand::{Pcg64, Rng};

    #[test]
    fn capability() {
        const ROUNDS: u32 = 1_000;
        let mut rng = Pcg64::new_seed(123456789);
        for _ in 0..ROUNDS {
            // fields individually generated
            {
                let addr = Address(rng.generate());
                let start = Address(rng.generate());
                let endb = Address(rng.generate());
                let perms = Permissions::from_bits_truncate(rng.generate());
                let cap = Capability::new(addr, start, endb, perms);
                assert_eq!(cap, Capability::from_ugran(cap.to_ugran()));
            }

            // ugran randomly generated
            {
                let ugran: UGran = rng.generate();
                assert_eq!(ugran, Capability::from_ugran(ugran).to_ugran());
            }
        }
    }
}

mod revoke {
    use crate::abi::{Align, Layout};
    use crate::mem::Memory;
    use crate::registers::Register;
    use crate::{alloc, revoke};

    #[test]
    fn endb_is_harmless() -> anyhow::Result<()> {
        let mut mem = Memory::new(16, 0, [].iter())?;
        let root_cap = mem.regs.read(&mem.tags, Register::Z0 as _)?;
        let ation = alloc::alloc(
            root_cap,
            Layout {
                size: 8,
                align: Align::new(1).unwrap(),
            },
            &mut mem,
        )?;
        mem.regs.write(&mut mem.tags, Register::T0 as _, ation)?;
        revoke::by_bounds(&mut mem, ation.endb(), ation.endb().add(1))?;
        revoke::by_bounds(&mut mem, ation.start().sub(1), ation.start())?;
        let new_ation = mem.regs.read(&mem.tags, Register::T0 as _)?;
        assert!(new_ation.is_valid());
        Ok(())
    }
}
