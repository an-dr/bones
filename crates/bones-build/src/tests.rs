use super::*;

#[test]
fn the_last_wasm_artifact_is_found_among_cargo_messages() {
    let messages = r#"{"reason":"compiler-artifact","filenames":["/t/deps/libserde.rlib"]}
{"reason":"compiler-artifact","filenames":["/t/release/pong.wasm"]}
{"reason":"build-finished","success":true}"#;
    assert_eq!(
        wasm_artifact(messages),
        Some(PathBuf::from("/t/release/pong.wasm"))
    );
}

#[test]
fn no_wasm_artifact_is_none() {
    assert_eq!(
        wasm_artifact(r#"{"reason":"build-finished","success":true}"#),
        None
    );
}

#[test]
fn a_build_of_a_missing_directory_fails_with_the_path() {
    let error = Build::new("no/such/extension").run().unwrap_err();
    assert!(
        error.contains("no/such/extension") || error.contains("no\\such\\extension"),
        "{error}"
    );
}

#[test]
fn a_real_extension_builds_to_its_wasm() {
    let hello = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/extensions/hello-demo");
    let wasm = Build::new(hello).run().unwrap();
    assert_eq!(wasm.file_name().unwrap(), "hello_demo.wasm");
    assert!(wasm.is_file());
}
