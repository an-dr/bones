use super::*;

// Built by extensions/hello/build.ps1 (see its README).
const HELLO_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../extensions/hello/target/wasm32-wasip2/release"
);

#[test]
fn find_wasm_files_finds_only_wasm_extensions_sorted() {
    let files = find_wasm_files(Path::new(HELLO_DIR));
    assert!(
        files.iter().all(|f| f.extension().unwrap() == "wasm"),
        "expected only .wasm files, got {files:?}"
    );
    assert!(
        files.iter().any(|f| f.file_stem().unwrap() == "hello"),
        "expected hello.wasm in {files:?} — run extensions/hello/build.ps1 first"
    );
    let mut sorted = files.clone();
    sorted.sort();
    assert_eq!(files, sorted);
}

#[test]
fn find_wasm_files_on_a_missing_directory_is_empty_not_an_error() {
    assert_eq!(
        find_wasm_files(Path::new("no/such/directory")),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn derive_extension_name_is_the_file_stem() {
    assert_eq!(derive_extension_name(Path::new("/a/b/hello.wasm")), "hello");
}

#[test]
fn is_first_occurrence_accepts_a_name_once_and_rejects_a_repeat() {
    let mut seen = std::collections::HashSet::new();
    assert!(is_first_occurrence(&mut seen, "hello"));
    assert!(!is_first_occurrence(&mut seen, "hello"));
    assert!(is_first_occurrence(&mut seen, "keyecho"));
}
