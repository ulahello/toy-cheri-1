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
        for data in 0..(2u8.pow(Permissions::BITS as _)) {
            let perms = Permissions::from_data(data);
            assert_eq!(perms, Permissions::from_data(perms.to_data()));
        }
    }
}
