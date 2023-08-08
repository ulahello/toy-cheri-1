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

mod capability {
    use crate::capability::{Address, Capability, Permissions, TaggedCapability};

    #[test]
    fn is_bounded() {
        let normal = Capability::new(Address(8), Address(0), Address(16), Permissions::empty());
        assert!(normal.is_bounded_with_len(0));
        assert!(normal.is_bounded_with_len(8));
        assert!(!normal.is_bounded_with_len(9));
        assert!(!normal.is_bounded_with_len(18));

        let reverse = Capability::new(Address(8), Address(16), Address(0), Permissions::empty());
        assert!(!reverse.is_bounded_with_len(0));
        assert!(!reverse.is_bounded_with_len(8));
        assert!(!reverse.is_bounded_with_len(9));
        assert!(!reverse.is_bounded_with_len(18));

        let eq = Capability::new(Address(16), Address(0), Address(16), Permissions::empty());
        assert!(eq.is_bounded_with_len(0));
        assert!(!eq.is_bounded_with_len(8));
        assert!(!eq.is_bounded_with_len(9));
        assert!(!eq.is_bounded_with_len(18));

        let oob_right = normal.set_addr(Address(32));
        assert!(!oob_right.is_bounded_with_len(0));
        assert!(!oob_right.is_bounded_with_len(8));
        assert!(!oob_right.is_bounded_with_len(9));
        assert!(!oob_right.is_bounded_with_len(18));

        let oob_left = Capability::new(Address(0), Address(16), Address(32), Permissions::empty());
        assert!(!oob_left.is_bounded_with_len(0));
        assert!(!oob_left.is_bounded_with_len(8));
        assert!(!oob_left.is_bounded_with_len(9));
        assert!(!oob_left.is_bounded_with_len(18));
    }

    #[test]
    fn set_perms() {
        let mut cap = TaggedCapability::new(
            Capability::new(Address(0), Address(0), Address(16), Permissions::all()),
            true,
        );

        assert!(cap.set_perms(Permissions::READ).is_valid());
        assert!(cap.set_perms(Permissions::WRITE).is_valid());
        assert!(cap.set_perms(Permissions::EXEC).is_valid());
        assert!(cap
            .set_perms(Permissions::READ | Permissions::WRITE)
            .is_valid());
        assert!(cap
            .set_perms(Permissions::READ | Permissions::EXEC)
            .is_valid());
        assert!(cap
            .set_perms(Permissions::WRITE | Permissions::EXEC)
            .is_valid());
        assert!(cap.set_perms(Permissions::all()).is_valid());

        cap = cap.set_perms(Permissions::READ | Permissions::EXEC);

        assert!(cap.set_perms(Permissions::READ).is_valid());
        assert!(!cap.set_perms(Permissions::WRITE).is_valid());
        assert!(cap.set_perms(Permissions::EXEC).is_valid());
        assert!(!cap
            .set_perms(Permissions::READ | Permissions::WRITE)
            .is_valid());
        assert!(cap
            .set_perms(Permissions::READ | Permissions::EXEC)
            .is_valid());
        assert!(!cap
            .set_perms(Permissions::WRITE | Permissions::EXEC)
            .is_valid());
        assert!(!cap.set_perms(Permissions::all()).is_valid());
    }
}
