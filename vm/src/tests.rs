mod serde {
    use crate::capability::{Address, Capability, Permissions};
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
                let perms = Permissions::from_data(rng.generate());
                let cap = Capability::new(addr, start, endb, perms);
                assert_eq!(cap, Capability::from_ugran(cap.to_ugran()));
            }

            // ugran randomly generated
            {
                let cap = Capability::from_ugran(rng.generate());
                assert_eq!(cap, Capability::from_ugran(cap.to_ugran()));
            }
        }
    }

    #[test]
    fn perms() {
        const ROUNDS: u32 = 1_000;
        let mut rng = Pcg64::new_seed(123456789);
        for _ in 0..ROUNDS {
            // by randomly generating fields
            {
                let r = rng.generate();
                let w = rng.generate();
                let x = rng.generate();
                let perms = Permissions { r, w, x };
                assert_eq!(perms, Permissions::from_data(perms.to_data()));
            }

            // by randomly generating data
            {
                let perms = Permissions::from_data(rng.generate());
                assert_eq!(perms, Permissions::from_data(perms.to_data()));
            }
        }
    }
}
