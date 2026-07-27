#[test]
fn abi_profile_constants() {
    assert_eq!(crate::NODE_MODULE_VERSION, 137);
    assert_eq!(crate::NODE_VERSION, "24.3.0");
    assert_eq!(crate::ABI_PROFILE_LABEL, "node24-abi137");
}
