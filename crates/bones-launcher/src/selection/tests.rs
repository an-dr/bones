use super::*;
use std::fs;

fn make_version_dir(install_dir: &Path, name: &str) {
    fs::create_dir_all(install_dir.join(name)).unwrap();
}

#[test]
fn current_version_dir_is_none_without_any_version_folder() {
    let dir = tempfile::tempdir().unwrap();
    assert!(current_version_dir(dir.path()).is_none());
}

#[test]
fn a_stable_release_wins_over_a_more_recent_prerelease() {
    let dir = tempfile::tempdir().unwrap();
    make_version_dir(dir.path(), "1.2.3+build");
    make_version_dir(dir.path(), "1.2.3-rc.9~abc");
    assert_eq!(
        current_version_dir(dir.path())
            .unwrap()
            .file_name()
            .unwrap(),
        "1.2.3+build"
    );
}

#[test]
fn current_version_dir_picks_the_highest_numeric_version() {
    let dir = tempfile::tempdir().unwrap();
    make_version_dir(dir.path(), "1.2.0");
    make_version_dir(dir.path(), "1.10.0");
    make_version_dir(dir.path(), "1.9.0");

    let current = current_version_dir(dir.path()).unwrap();

    assert_eq!(current.file_name().unwrap(), "1.10.0");
}

#[test]
fn non_version_named_entries_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    make_version_dir(dir.path(), "1.2.0");
    make_version_dir(dir.path(), "updater");
    make_version_dir(dir.path(), "components");
    make_version_dir(dir.path(), "state");
    fs::write(dir.path().join("commits.exe"), b"launcher").unwrap();
    fs::write(dir.path().join("commits.log"), b"log").unwrap();

    let current = current_version_dir(dir.path()).unwrap();

    assert_eq!(current.file_name().unwrap(), "1.2.0");
}

#[test]
fn previous_version_dir_is_the_second_highest() {
    let dir = tempfile::tempdir().unwrap();
    make_version_dir(dir.path(), "1.2.0");
    make_version_dir(dir.path(), "1.3.0");
    make_version_dir(dir.path(), "1.1.0");

    let previous = previous_version_dir(dir.path()).unwrap();

    assert_eq!(previous.file_name().unwrap(), "1.2.0");
}

#[test]
fn previous_version_dir_is_none_with_only_one_version() {
    let dir = tempfile::tempdir().unwrap();
    make_version_dir(dir.path(), "1.2.0");

    assert!(previous_version_dir(dir.path()).is_none());
}

#[test]
fn a_disambiguated_suffix_still_orders_by_its_leading_version() {
    let dir = tempfile::tempdir().unwrap();
    make_version_dir(dir.path(), "1.2.0");
    make_version_dir(dir.path(), "1.10.0~a1b2c3d4");

    let current = current_version_dir(dir.path()).unwrap();

    assert_eq!(current.file_name().unwrap(), "1.10.0~a1b2c3d4");
}

#[test]
fn same_numeric_version_breaks_the_tie_by_modification_time() {
    // Two dev-build pushes that never bump their version collide on the
    // bare folder name and get a disambiguating suffix (see
    // extract_version / copy_version_from_dir); the one installed most
    // recently -- not the one enumerated first by the filesystem --
    // should be treated as current.
    let dir = tempfile::tempdir().unwrap();
    make_version_dir(dir.path(), "0.0.0~aaaaaaaa");
    std::thread::sleep(std::time::Duration::from_millis(20));
    make_version_dir(dir.path(), "0.0.0~bbbbbbbb");

    let current = current_version_dir(dir.path()).unwrap();

    assert_eq!(current.file_name().unwrap(), "0.0.0~bbbbbbbb");
}
