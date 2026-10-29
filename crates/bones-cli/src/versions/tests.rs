use super::*;

#[test]
fn the_abi_version_is_read_from_the_manifest() {
    assert_eq!(
        abi_from("[package]\nname = \"x\"\nversion = \"1.2.3\"\n"),
        Some("1.2.3")
    );
}

#[test]
fn the_built_in_abi_version_matches_the_sdk() {
    let sdk = include_str!("../../../bones-wasm-sdk/Cargo.toml");
    assert_eq!(abi_from(sdk), Some(abi()));
}
