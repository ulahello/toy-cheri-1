mod serde {
    use crate::capability::{Address, Capability, OType, Permissions};
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
                let otype = OType::new(rng.generate());
                let cap = Capability::new(addr, start, endb, perms, otype);
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
    use crate::capability::{Address, Capability, OType, Permissions, TaggedCapability};

    #[test]
    fn is_bounded() {
        let normal = Capability::new(
            Address(8),
            Address(0),
            Address(16),
            Permissions::empty(),
            OType::UNSEALED,
        );
        assert!(normal.is_bounded_with_len(0));
        assert!(normal.is_bounded_with_len(8));
        assert!(!normal.is_bounded_with_len(9));
        assert!(!normal.is_bounded_with_len(18));

        let reverse = Capability::new(
            Address(8),
            Address(16),
            Address(0),
            Permissions::empty(),
            OType::UNSEALED,
        );
        assert!(!reverse.is_bounded_with_len(0));
        assert!(!reverse.is_bounded_with_len(8));
        assert!(!reverse.is_bounded_with_len(9));
        assert!(!reverse.is_bounded_with_len(18));

        let eq = Capability::new(
            Address(16),
            Address(0),
            Address(16),
            Permissions::empty(),
            OType::UNSEALED,
        );
        assert!(eq.is_bounded_with_len(0));
        assert!(!eq.is_bounded_with_len(8));
        assert!(!eq.is_bounded_with_len(9));
        assert!(!eq.is_bounded_with_len(18));

        let oob_right = normal.set_addr(Address(32));
        assert!(!oob_right.is_bounded_with_len(0));
        assert!(!oob_right.is_bounded_with_len(8));
        assert!(!oob_right.is_bounded_with_len(9));
        assert!(!oob_right.is_bounded_with_len(18));

        let oob_left = Capability::new(
            Address(0),
            Address(16),
            Address(32),
            Permissions::empty(),
            OType::UNSEALED,
        );
        assert!(!oob_left.is_bounded_with_len(0));
        assert!(!oob_left.is_bounded_with_len(8));
        assert!(!oob_left.is_bounded_with_len(9));
        assert!(!oob_left.is_bounded_with_len(18));
    }

    #[test]
    fn set_perms() {
        // TODO: automate

        let mut cap = TaggedCapability::new(
            Capability::new(
                Address(0),
                Address(0),
                Address(16),
                Permissions::all(),
                OType::UNSEALED,
            ),
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

    #[test]
    fn set_bounds() {
        let cap = TaggedCapability::new(
            Capability::new(
                Address(8),
                Address(8),
                Address(16),
                Permissions::all(),
                OType::UNSEALED,
            ),
            true,
        );
        assert!(cap.set_bounds(Address(8), Address(16)).is_valid());
        assert!(cap.set_bounds(Address(9), Address(10)).is_valid());
        assert!(!cap.set_bounds(Address(7), Address(15)).is_valid());
        assert!(!cap.set_bounds(Address(9), Address(17)).is_valid());
        assert!(!cap.set_bounds(Address(0), Address(2)).is_valid());
        assert!(cap.set_bounds(Address(16), Address(16)).is_valid());
        assert!(!cap.set_bounds(Address(16), Address(17)).is_valid());
        assert!(!cap.set_bounds(Address(16), Address(8)).is_valid());
    }

    #[test]
    fn seal() {
        for (cap, sealer, expect_ok) in [
            (
                TaggedCapability::new(
                    Capability::new(
                        Address(0),
                        Address(0),
                        Address(16),
                        Permissions::READ | Permissions::WRITE,
                        OType::UNSEALED,
                    ),
                    true,
                ),
                TaggedCapability::new(
                    Capability::new(
                        Address(256),
                        Address(256),
                        Address(320),
                        Permissions::SEAL,
                        OType::UNSEALED,
                    ),
                    true,
                ),
                true,
            ),
            (
                TaggedCapability::INVALID,
                TaggedCapability::new(
                    Capability::new(
                        Address(256),
                        Address(256),
                        Address(320),
                        Permissions::SEAL,
                        OType::UNSEALED,
                    ),
                    true,
                ),
                false,
            ),
            (
                TaggedCapability::new(
                    Capability::new(
                        Address(0),
                        Address(0),
                        Address(16),
                        Permissions::all(),
                        OType::UNSEALED,
                    ),
                    true,
                ),
                TaggedCapability::INVALID,
                false,
            ),
            (
                TaggedCapability::new(
                    Capability::new(
                        Address(0),
                        Address(0),
                        Address(16),
                        Permissions::READ | Permissions::WRITE,
                        OType::UNSEALED,
                    ),
                    true,
                ),
                TaggedCapability::new(
                    Capability::new(
                        Address(64), // not aligned to OType::VALID_ALIGN
                        Address(128),
                        Address(128),
                        Permissions::SEAL,
                        OType::UNSEALED,
                    ),
                    true,
                ),
                false,
            ),
        ] {
            let sealed = cap.seal(sealer);
            _ = dbg!(cap, sealer, expect_ok, sealed);
            assert_eq!(sealed.is_valid(), expect_ok);
            if expect_ok {
                assert!(sealed.otype().is_sealed());
            }
        }
    }

    #[test]
    fn unseal() {
        for (cap, unsealer, expect_ok) in [
            (
                TaggedCapability::new(
                    Capability::new(
                        Address(0),
                        Address(0),
                        Address(16),
                        Permissions::READ | Permissions::WRITE,
                        OType::try_new(Address(256)).unwrap(),
                    ),
                    true,
                ),
                TaggedCapability::new(
                    Capability::new(
                        Address(256),
                        Address(256),
                        Address(320),
                        Permissions::UNSEAL,
                        OType::UNSEALED,
                    ),
                    true,
                ),
                true,
            ),
            (
                TaggedCapability::INVALID,
                TaggedCapability::new(
                    Capability::new(
                        Address(256),
                        Address(256),
                        Address(320),
                        Permissions::UNSEAL,
                        OType::UNSEALED,
                    ),
                    true,
                ),
                false,
            ),
            (
                TaggedCapability::new(
                    Capability::new(
                        Address(0),
                        Address(0),
                        Address(16),
                        Permissions::all(),
                        OType::UNSEALED,
                    ),
                    true,
                ),
                TaggedCapability::INVALID,
                false,
            ),
            (
                TaggedCapability::new(
                    Capability::new(
                        Address(0),
                        Address(0),
                        Address(16),
                        Permissions::READ | Permissions::WRITE,
                        OType::try_new(Address(256)).unwrap(),
                    ),
                    true,
                ),
                TaggedCapability::new(
                    Capability::new(
                        Address(64), // not aligned to OType::VALID_ALIGN
                        Address(128),
                        Address(128),
                        Permissions::SEAL,
                        OType::UNSEALED,
                    ),
                    true,
                ),
                false,
            ),
        ] {
            let unsealed = cap.unseal(unsealer);
            _ = dbg!(cap, unsealer, expect_ok, unsealed);
            assert_eq!(unsealed.is_valid(), expect_ok);
            if expect_ok {
                assert!(unsealed.otype().is_unsealed());
            }
        }
    }
}
