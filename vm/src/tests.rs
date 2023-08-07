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
