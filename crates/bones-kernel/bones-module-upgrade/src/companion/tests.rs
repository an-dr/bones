use std::io::Write;
use std::path::{Path, PathBuf};

use super::*;

/// Each test redirects its own variable, so tests running in parallel never
/// share an install location.
fn identity(env_var: &'static str) -> AppIdentity {
    AppIdentity {
        install_dir_name: ".companion-probe",
        install_env_var: env_var,
        launcher_stem: "buddy",
        app_stem: "buddy-app",
    }
}

/// An update payload as a companion's release publishes it: one version
/// folder's contents at the root, the launcher under its payload name.
fn payload(identity: &AppIdentity, dir: &Path, marker: &[u8]) -> PathBuf {
    let path = dir.join("buddy.zip");
    let file = std::fs::File::create(&path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
    for (name, contents) in [
        (identity.launcher_payload_exe(), b"launcher".as_slice()),
        (identity.app_exe(), marker),
        ("page.html".to_string(), b"<html></html>".as_slice()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap();
    path
}

fn sha256_hex(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// An install as the companion's own `--install` leaves it.
fn existing_install(identity: &AppIdentity, install_dir: &Path, version: &str) {
    std::fs::create_dir_all(install_dir.join(version)).unwrap();
    std::fs::write(install_dir.join(identity.launcher_exe()), b"old launcher").unwrap();
    std::fs::write(install_dir.join(version).join(identity.app_exe()), b"old").unwrap();
}

#[test]
fn a_fresh_install_is_shaped_like_the_companion_installed_itself() {
    let identity = identity("COMPANION_TEST_FRESH_DIR");
    let root = tempfile::tempdir().unwrap();
    let install_dir = root.path().join("app");
    std::env::set_var(identity.install_env_var, &install_dir);
    let archive = payload(&identity, root.path(), b"1.8.1");

    let outcome = install_companion(
        &identity,
        &archive,
        "1.8.1",
        Some(&sha256_hex(&archive)),
        Policy::IfAbsent,
    )
    .unwrap();

    assert_eq!(
        outcome,
        Outcome::Installed {
            version: "1.8.1".into()
        }
    );
    assert_eq!(
        std::fs::read(install_dir.join(identity.launcher_exe())).unwrap(),
        b"launcher"
    );
    assert_eq!(
        current_version_dir(&install_dir),
        Some(install_dir.join("1.8.1"))
    );
    assert_eq!(
        std::fs::read(install_dir.join("1.8.1").join(identity.app_exe())).unwrap(),
        b"1.8.1"
    );
    // The payload stays in the version folder: it is what a later start
    // refreshes the entry point from.
    assert!(install_dir
        .join("1.8.1")
        .join(identity.launcher_payload_exe())
        .is_file());
}

#[test]
fn an_existing_install_is_left_untouched_when_absent_is_required() {
    let identity = identity("COMPANION_TEST_ABSENT_DIR");
    let root = tempfile::tempdir().unwrap();
    let install_dir = root.path().join("app");
    existing_install(&identity, &install_dir, "1.0.0");
    std::env::set_var(identity.install_env_var, &install_dir);
    let archive = payload(&identity, root.path(), b"9.0.0");

    let outcome = install_companion(&identity, &archive, "9.0.0", None, Policy::IfAbsent).unwrap();

    assert_eq!(
        outcome,
        Outcome::AlreadyPresent {
            version: "1.0.0".into()
        }
    );
    assert_eq!(
        current_version_dir(&install_dir),
        Some(install_dir.join("1.0.0"))
    );
    assert_eq!(
        std::fs::read(install_dir.join(identity.launcher_exe())).unwrap(),
        b"old launcher"
    );
}

#[test]
fn an_older_install_gains_a_version_folder_when_newer_is_allowed() {
    let identity = identity("COMPANION_TEST_OLDER_DIR");
    let root = tempfile::tempdir().unwrap();
    let install_dir = root.path().join("app");
    existing_install(&identity, &install_dir, "1.0.0");
    std::env::set_var(identity.install_env_var, &install_dir);
    let archive = payload(&identity, root.path(), b"2.0.0");

    let outcome = install_companion(&identity, &archive, "2.0.0", None, Policy::IfNewer).unwrap();

    assert_eq!(
        outcome,
        Outcome::Installed {
            version: "2.0.0".into()
        }
    );
    assert_eq!(
        current_version_dir(&install_dir),
        Some(install_dir.join("2.0.0"))
    );
    assert!(install_dir.join("1.0.0").is_dir(), "kept for rollback");
}

#[test]
fn a_newer_install_is_left_alone_when_newer_is_allowed() {
    let identity = identity("COMPANION_TEST_NEWER_DIR");
    let root = tempfile::tempdir().unwrap();
    let install_dir = root.path().join("app");
    existing_install(&identity, &install_dir, "3.0.0");
    std::env::set_var(identity.install_env_var, &install_dir);
    let archive = payload(&identity, root.path(), b"2.0.0");

    let outcome = install_companion(&identity, &archive, "2.0.0", None, Policy::IfNewer).unwrap();

    assert!(matches!(outcome, Outcome::Skipped { .. }), "{outcome:?}");
    assert!(!install_dir.join("2.0.0").exists());
}

#[test]
fn companion_comparison_preserves_prerelease_and_ignores_build_metadata() {
    let identity = identity("COMPANION_TEST_SEMVER_DIR");
    let root = tempfile::tempdir().unwrap();
    let install_dir = root.path().join("app");
    existing_install(&identity, &install_dir, "1.2.3-rc.1+old~hash");
    std::env::set_var(identity.install_env_var, &install_dir);
    let archive = payload(&identity, root.path(), b"new");
    let result =
        install_companion(&identity, &archive, "1.2.3-rc.2+new", None, Policy::IfNewer).unwrap();
    assert!(matches!(result, Outcome::Installed { .. }));
    let result = install_companion(
        &identity,
        &archive,
        "1.2.3-rc.2+other",
        None,
        Policy::IfNewer,
    )
    .unwrap();
    assert!(matches!(result, Outcome::Skipped { .. }));
    let result = install_companion(&identity, &archive, "1.2.3", None, Policy::IfNewer).unwrap();
    assert!(matches!(result, Outcome::Installed { .. }));
}

#[test]
fn a_corrupt_archive_is_an_error() {
    let identity = identity("COMPANION_TEST_CORRUPT_DIR");
    let root = tempfile::tempdir().unwrap();
    let install_dir = root.path().join("app");
    std::env::set_var(identity.install_env_var, &install_dir);
    let archive = root.path().join("broken.zip");
    std::fs::write(&archive, b"not a zip at all").unwrap();

    let result = install_companion(&identity, &archive, "1.0.0", None, Policy::IfAbsent);

    assert!(result.is_err());
    assert!(!install_dir.join(identity.launcher_exe()).exists());
}

#[test]
fn a_checksum_mismatch_writes_nothing() {
    let identity = identity("COMPANION_TEST_CHECKSUM_DIR");
    let root = tempfile::tempdir().unwrap();
    let install_dir = root.path().join("app");
    std::env::set_var(identity.install_env_var, &install_dir);
    let archive = payload(&identity, root.path(), b"1.0.0");

    let result = install_companion(&identity, &archive, "1.0.0", Some("0000"), Policy::IfAbsent);

    assert!(result.is_err());
    assert!(!install_dir.exists());
}
