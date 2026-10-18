use std::fs;
use std::path::PathBuf;

use super::{sha256_hex, write_zip};

/// A fresh scratch directory under the system temp dir, unique per test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-archive-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn entries_sit_under_the_prefix_in_sorted_order() {
    let dir = scratch("prefix");
    let source = dir.join("dist");
    fs::create_dir_all(source.join("wit")).unwrap();
    fs::write(source.join("b.txt"), "b").unwrap();
    fs::write(source.join("a.txt"), "a").unwrap();
    fs::write(source.join("wit/x.wit"), "x").unwrap();
    let archive = dir.join("out.zip");

    write_zip(&source, "bundle", &archive).unwrap();

    let zip = zip::ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
    let names: Vec<_> = zip.file_names().collect();
    assert_eq!(
        names,
        [
            "bundle/a.txt",
            "bundle/b.txt",
            "bundle/wit/",
            "bundle/wit/x.wit"
        ]
    );
}

#[cfg(unix)]
#[test]
fn an_executable_unpacks_executable() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("mode");
    let source = dir.join("dist");
    fs::create_dir_all(&source).unwrap();
    let exe = source.join("bones");
    fs::write(&exe, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    let archive = dir.join("out.zip");

    write_zip(&source, "", &archive).unwrap();

    let mut zip = zip::ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
    let mode = zip.by_name("bones").unwrap().unix_mode().unwrap();
    assert_eq!(mode & 0o777, 0o755);
}

#[test]
fn sha256_hex_is_lowercase_and_known() {
    let dir = scratch("sha");
    let file = dir.join("abc");
    fs::write(&file, "abc").unwrap();
    assert_eq!(
        sha256_hex(&file).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
