mod serde {
    use crate::capability::Permissions;

    #[test]
    fn perms() {
        for data in 0..(2u8.pow(Permissions::BITS as _)) {
            let perms = Permissions::from_data(data);
            assert_eq!(perms, Permissions::from_data(perms.to_data()));
        }
    }
}
