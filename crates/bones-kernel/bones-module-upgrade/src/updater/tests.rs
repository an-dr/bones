use std::collections::HashMap;
use std::io::Write;

use base64::Engine;

use super::*;
#[cfg(unix)]
use crate::install::mark_executables;
use crate::{current_version_dir, previous_version_dir, remove_version_dir, PLATFORM};

/// Serves fixed responses, so the flow runs without a network.
struct StubFetch(HashMap<String, Vec<u8>>);

impl Fetch for StubFetch {
    fn fetch_url(&self, url: &str) -> Result<Option<String>, String> {
        Ok(self.0.get(url).map(|bytes| {
            format!(
                "application/octet-stream;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        }))
    }
}

const MANIFEST_URL: &str = "https://example.test/app-windows-x64.json";
const ARCHIVE_URL: &str = "https://example.test/app-1.2.0.zip";

fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        for (name, contents) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }
    buffer
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A published release of `version`, with a manifest whose checksum is
/// `sha256` or, by default, the archive's real one.
fn release(version: &str, sha256: Option<&str>) -> StubFetch {
    let archive = zip_bytes(&[("app.exe", version.as_bytes())]);
    let sha256 = sha256.map_or_else(|| sha256_hex(&archive), str::to_string);
    let manifest =
        format!(r#"{{"version":"{version}","url":"{ARCHIVE_URL}","sha256":"{sha256}"}}"#);
    StubFetch(HashMap::from([
        (MANIFEST_URL.to_string(), manifest.into_bytes()),
        (ARCHIVE_URL.to_string(), archive),
    ]))
}

fn source() -> ReleaseSource {
    ReleaseSource::Url(MANIFEST_URL.into())
}

#[test]
fn check_returns_the_manifest_of_a_newer_release() {
    let manifest = check(&release("1.2.0", None), &source(), "1.1.0")
        .unwrap()
        .expect("1.2.0 is newer");

    assert_eq!(manifest.version, "1.2.0");
}

#[test]
fn check_uses_semver_precedence_and_reports_invalid_versions() {
    assert!(check(&release("1.2.0", None), &source(), "1.2.0-rc.1")
        .unwrap()
        .is_some());
    assert!(check(&release("1.2.0-rc.2", None), &source(), "1.2.0-rc.1")
        .unwrap()
        .is_some());
    assert!(check(&release("1.2.0-rc.1", None), &source(), "1.2.0")
        .unwrap()
        .is_none());
    assert!(check(&release("1.2.0+z", None), &source(), "1.2.0+a")
        .unwrap()
        .is_none());
    assert!(check(&release("1.2", None), &source(), "1.0.0").is_err());
    assert!(check(&release("1.2.0", None), &source(), "invalid").is_err());
}

#[test]
fn check_reports_up_to_date_for_the_same_or_an_older_release() {
    assert_eq!(
        check(&release("1.2.0", None), &source(), "1.2.0").unwrap(),
        None
    );
    assert_eq!(
        check(&release("1.2.0", None), &source(), "1.3.0").unwrap(),
        None
    );
}

#[test]
fn check_fails_when_no_manifest_is_published() {
    assert!(check(&StubFetch(HashMap::new()), &source(), "1.0.0").is_err());
}

#[test]
fn a_checksum_mismatch_fails_without_staging_anything() {
    let install = tempfile::tempdir().unwrap();
    let fetch = release("1.2.0", Some("00"));
    let manifest = check(&fetch, &source(), "1.1.0").unwrap().unwrap();

    assert!(stage(&fetch, &manifest, install.path()).is_err());

    assert!(current_version_dir(install.path()).is_none());
    assert_eq!(std::fs::read_dir(install.path()).unwrap().count(), 0);
}

#[test]
fn a_staged_release_is_what_the_launcher_starts_next() {
    let install = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(install.path().join("1.1.0")).unwrap();
    let fetch = release("1.2.0", None);
    let manifest = check(&fetch, &source(), "1.1.0").unwrap().unwrap();

    let staged = stage(&fetch, &manifest, install.path()).unwrap();

    assert_eq!(staged, install.path().join("1.2.0"));
    assert_eq!(current_version_dir(install.path()), Some(staged.clone()));
    assert_eq!(std::fs::read(staged.join("app.exe")).unwrap(), b"1.2.0");
}

#[test]
fn a_staged_release_that_fails_to_start_rolls_back_to_the_previous_one() {
    let install = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(install.path().join("1.1.0")).unwrap();
    let fetch = release("1.2.0", None);
    let manifest = check(&fetch, &source(), "1.1.0").unwrap().unwrap();
    let staged = stage(&fetch, &manifest, install.path()).unwrap();

    // What the launcher's `--rollback` does: drop the newest folder.
    assert_eq!(
        previous_version_dir(install.path()),
        Some(install.path().join("1.1.0"))
    );
    remove_version_dir(&staged).unwrap();

    assert_eq!(
        current_version_dir(install.path()),
        Some(install.path().join("1.1.0"))
    );
}

fn identity(env_var: &'static str) -> AppIdentity {
    AppIdentity {
        install_dir_name: ".updater-probe",
        install_env_var: env_var,
        launcher_stem: "probe",
        app_stem: "probe-app",
    }
}

#[test]
fn install_places_a_first_build_straight_into_the_install_location() {
    let identity = identity("UPDATER_TEST_FRESH_DIR");
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("build");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join(identity.launcher_exe()), b"launcher").unwrap();
    std::fs::write(source.join("probe-app.exe"), b"app").unwrap();
    let install_dir = root.path().join("app");
    std::env::set_var(identity.install_env_var, &install_dir);

    let fresh = install(&identity, &source, "1.0.0").unwrap();

    assert!(fresh);
    assert!(install_dir.join(identity.launcher_exe()).is_file());
    assert!(install_dir.join("1.0.0/probe-app.exe").is_file());
}

#[test]
fn install_beside_an_existing_launcher_adds_a_version_folder() {
    let identity = identity("UPDATER_TEST_EXISTING_DIR");
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("build");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("probe-app.exe"), b"new app").unwrap();
    let install_dir = root.path().join("app");
    std::fs::create_dir_all(&install_dir).unwrap();
    std::fs::write(install_dir.join(identity.launcher_exe()), b"installed").unwrap();
    std::env::set_var(identity.install_env_var, &install_dir);

    let fresh = install(&identity, &source, "2.0.0").unwrap();

    assert!(!fresh);
    assert_eq!(
        std::fs::read(install_dir.join(identity.launcher_exe())).unwrap(),
        b"installed"
    );
    assert!(install_dir.join("2.0.0/probe-app.exe").is_file());
}

#[test]
fn install_refuses_to_copy_an_installed_version_folder_into_itself() {
    let identity = identity("UPDATER_TEST_SELF_DIR");
    let root = tempfile::tempdir().unwrap();
    let version_dir = root.path().join("1.0.0");
    std::fs::create_dir_all(&version_dir).unwrap();
    std::env::set_var(identity.install_env_var, root.path());

    assert!(install(&identity, &version_dir, "1.0.0").is_err());
}

#[test]
fn an_empty_override_means_the_default_source() {
    let github = ReleaseSource::GitHub {
        owner: "an-dr".into(),
        repo: "commits".into(),
        app: "commits".into(),
    };

    assert_eq!(ReleaseSource::with_override(github.clone(), "  "), github);
    assert_eq!(
        ReleaseSource::with_override(github, " https://example.test/m.json "),
        ReleaseSource::Url("https://example.test/m.json".into())
    );
}

#[test]
fn a_github_source_reads_this_platforms_manifest_on_the_latest_release() {
    let github = ReleaseSource::GitHub {
        owner: "an-dr".into(),
        repo: "commits".into(),
        app: "commits".into(),
    };

    assert_eq!(
        github.manifest_url(),
        PLATFORM.map(|platform| format!(
            "https://github.com/an-dr/commits/releases/latest/download/commits-{platform}.json"
        ))
    );
}

#[test]
#[cfg(unix)]
fn staged_executables_become_runnable_and_other_files_do_not() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("app"), b"binary").unwrap();
    std::fs::write(dir.path().join("page.html"), b"page").unwrap();
    for name in ["app", "page.html"] {
        std::fs::set_permissions(
            dir.path().join(name),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
    }

    mark_executables(dir.path()).unwrap();

    let mode = |name: &str| {
        std::fs::metadata(dir.path().join(name))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode("app"), 0o755);
    assert_eq!(mode("page.html"), 0o644);
}
