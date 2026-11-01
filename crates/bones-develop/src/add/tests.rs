use super::*;

#[test]
fn options_come_in_any_order_and_the_submodule_path_is_optional() {
    let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(parse_options(&args(&[])).unwrap(), (None, None));
    assert_eq!(
        parse_options(&args(&["--submodule", "--tag", "v3.1.0"])).unwrap(),
        (Some("v3.1.0".into()), Some("bones".into()))
    );
    assert_eq!(
        parse_options(&args(&["--tag", "v3.1.0", "--submodule", "vendor/bones"])).unwrap(),
        (Some("v3.1.0".into()), Some("vendor/bones".into()))
    );
    assert!(parse_options(&args(&["--tag"])).is_err());
}

#[test]
fn a_submodule_pin_is_a_path_into_it_and_a_git_pin_names_the_tag() {
    assert_eq!(
        Pin::Path("bones".into()).args("bones-kernel", "v3.1.0"),
        ["bones-kernel", "--path", "bones/crates/bones-kernel"]
    );
    assert_eq!(
        Pin::Git("https://x".into()).args("bones-wasm-sdk", "abi-v1.3.0"),
        [
            "bones-wasm-sdk",
            "--git",
            "https://x",
            "--tag",
            "abi-v1.3.0"
        ]
    );
}
