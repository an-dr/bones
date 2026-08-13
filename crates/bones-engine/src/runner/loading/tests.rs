use super::*;

// Built by crates/bones-extension-hello/build.ps1 (see its README).
const HELLO_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/bones-extension-hello/target/wasm32-wasip2/release"
);

#[test]
fn find_wasm_files_finds_only_wasm_extensions_sorted() {
    let files = find_wasm_files(Path::new(HELLO_DIR));
    assert!(
        files.iter().all(|f| f.extension().unwrap() == "wasm"),
        "expected only .wasm files, got {files:?}"
    );
    assert!(
        files.iter().any(|f| f.file_stem().unwrap() == "bones_extension_hello"),
        "expected bones_extension_hello.wasm in {files:?} — run crates/bones-extension-hello/build.ps1 first"
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
fn find_wasm_files_recurses_into_group_directories() {
    let root = std::env::temp_dir().join("bones-recursive-extension-catalog");
    let nested = root.join("levels");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(root.join("menu.wasm"), []).unwrap();
    std::fs::write(nested.join("level_one.wasm"), []).unwrap();

    let files = find_wasm_files(&root);

    std::fs::remove_dir_all(root).ok();
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|path| path.ends_with("menu.wasm")));
    assert!(files.iter().any(|path| path.ends_with("level_one.wasm")));
}

#[cfg(unix)]
#[test]
fn find_wasm_files_does_not_follow_directory_symlinks() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join("bones-symlink-extension-catalog");
    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::write(root.join("nested/level.wasm"), []).unwrap();
    symlink(&root, root.join("nested/cycle")).unwrap();

    let files = find_wasm_files(&root);

    std::fs::remove_dir_all(root).ok();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("level.wasm"));
}

#[test]
fn derive_extension_name_is_the_file_stem() {
    assert_eq!(derive_extension_name(Path::new("/a/b/hello.wasm")), "hello");
}
