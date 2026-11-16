#[test]
fn stores_loads_and_forgets() {
    let name = "vaxelium-test-account";
    super::forget(name);
    assert_eq!(super::load(name), None);
    super::store(name, "secret-token").expect("keychain available");
    assert_eq!(super::load(name).as_deref(), Some("secret-token"));
    super::forget(name);
    assert_eq!(super::load(name), None);
}
