use super::*;

fn install_with_versions(names: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in names {
        std::fs::create_dir_all(dir.path().join(name)).unwrap();
    }
    dir
}

#[test]
fn rollback_drops_the_newest_version_so_the_previous_one_becomes_current() {
    let dir = install_with_versions(&["1.2.0", "1.3.0"]);

    let text = rollback(dir.path()).unwrap();

    assert!(!dir.path().join("1.3.0").exists());
    assert!(dir.path().join("1.2.0").exists());
    assert_eq!(
        current_version_dir(dir.path())
            .unwrap()
            .file_name()
            .unwrap(),
        "1.2.0"
    );
    assert!(text.contains("removed 1.3.0"), "{text}");
}

#[test]
fn rollback_refuses_to_leave_the_entry_point_with_nothing_to_start() {
    let dir = install_with_versions(&["1.3.0"]);

    let error = rollback(dir.path()).unwrap_err();

    assert!(error.contains("nothing to roll back to"), "{error}");
    assert!(
        dir.path().join("1.3.0").exists(),
        "the only version must survive"
    );
}

#[test]
fn rollback_reports_an_empty_install_rather_than_panicking() {
    let dir = tempfile::tempdir().unwrap();

    let error = rollback(dir.path()).unwrap_err();

    assert!(error.contains("no version is installed"), "{error}");
}
