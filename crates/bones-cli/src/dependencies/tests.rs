use super::*;

#[test]
fn tagged_dependencies_pin_the_abi_line_for_the_sdk_and_the_engine_line_for_the_rest() {
    let dependencies = Dependencies::tagged();
    assert!(dependencies
        .sdk
        .contains(&format!("abi-v{}", versions::abi())));
    assert!(dependencies
        .test
        .contains(&format!("\"v{}\"", versions::ENGINE)));
    assert!(dependencies.kernel.contains("\"wasm\""));
}

#[test]
fn local_dependencies_use_forward_slashes() {
    let dependencies = Dependencies::local(Path::new(r"C:\code\bones"));
    assert_eq!(
        dependencies.sdk,
        r#"bones-wasm-sdk = { path = "C:/code/bones/crates/bones-wasm-sdk" }"#
    );
}
