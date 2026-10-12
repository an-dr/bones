use super::*;
use std::io::Write;

#[test]
fn a_version_is_selectable_only_after_its_build_finishes() {
    let dir = tempfile::tempdir().unwrap();
    let published = publish_version(dir.path(), "2.0.0", "hash", |scratch| {
        fs::write(scratch.join("app.exe"), b"ready").unwrap();
        assert!(crate::current_version_dir(dir.path()).is_none());
        Ok(())
    })
    .unwrap();
    assert_eq!(crate::current_version_dir(dir.path()), Some(published));
}

#[test]
fn a_failed_build_leaves_the_previous_version_and_no_candidate() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("1.0.0")).unwrap();
    let result = publish_version(dir.path(), "2.0.0", "hash", |scratch| {
        fs::write(scratch.join("partial.exe"), b"partial").unwrap();
        Err("build failed".into())
    });
    assert_eq!(result.unwrap_err(), "build failed");
    assert_eq!(
        crate::current_version_dir(dir.path()),
        Some(dir.path().join("1.0.0"))
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn repeated_payloads_never_clear_an_existing_version_folder() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = zip_bytes(&[("app.exe", b"ready")]);
    extract_version(&bytes, dir.path(), "1.0.0").unwrap();
    let second = extract_version(&bytes, dir.path(), "1.0.0").unwrap();
    fs::write(second.join("preserve.txt"), b"existing").unwrap();
    let third = extract_version(&bytes, dir.path(), "1.0.0").unwrap();
    assert_ne!(second, third);
    assert_eq!(fs::read(second.join("preserve.txt")).unwrap(), b"existing");
}

#[test]
fn a_release_version_cannot_escape_the_installation_directory() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = zip_bytes(&[("app.exe", b"ready")]);
    for version in [
        "../2.0",
        "2.0/child",
        "2.0-dev/child",
        "2.0-dev\\child",
        "",
        "unknown",
        "1.2.0~hash",
        "1.2",
        "01.2.3",
    ] {
        assert!(extract_version(&bytes, dir.path(), version).is_err());
    }
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn repeated_prereleases_preserve_their_complete_version() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = zip_bytes(&[("app.exe", b"ready")]);
    let version = "1.2.3-rc.1+build.7";
    extract_version(&bytes, dir.path(), version).unwrap();
    let second = extract_version(&bytes, dir.path(), version).unwrap();
    assert!(second
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("1.2.3-rc.1+build.7~"));
    assert_eq!(
        bones_versioning::parse_folder_name(second.file_name().unwrap().to_str().unwrap())
            .unwrap()
            .to_string(),
        version
    );
}

/// Deliberately not this repository's own names: every path these tests
/// build comes from the identity, so a value left hardcoded in the code
/// under test fails here rather than passing by coincidence.
fn identity() -> AppIdentity {
    AppIdentity {
        install_dir_name: ".probe",
        install_env_var: "PROBE_INSTALL_DIR",
        launcher_stem: "probe",
        app_stem: "probe-app",
    }
}

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

#[test]
fn extract_version_creates_a_version_folder_named_after_the_version() {
    let dir = tempfile::tempdir().unwrap();
    let install_dir = dir.path().join("app");
    let archive = zip_bytes(&[
        ("commits-app.exe", b"the app"),
        ("page.html", b"<html></html>"),
    ]);

    let version_dir = extract_version(&archive, &install_dir, "1.3.0").unwrap();

    assert_eq!(version_dir, install_dir.join("1.3.0"));
    assert_eq!(
        fs::read(version_dir.join("commits-app.exe")).unwrap(),
        b"the app"
    );
    assert_eq!(
        fs::read(version_dir.join("page.html")).unwrap(),
        b"<html></html>"
    );
}

#[test]
fn extract_version_keeps_components_inside_the_version_folder() {
    let dir = tempfile::tempdir().unwrap();
    let install_dir = dir.path().join("app");
    let archive = zip_bytes(&[
        ("commits-app.exe", b"the app"),
        ("components/commits.wasm", b"component"),
    ]);

    let version_dir = extract_version(&archive, &install_dir, "1.3.0").unwrap();

    assert_eq!(
        fs::read(version_dir.join("components/commits.wasm")).unwrap(),
        b"component"
    );
    assert!(!install_dir.join("components").exists());
}

#[test]
fn extract_version_disambiguates_a_version_string_collision() {
    let dir = tempfile::tempdir().unwrap();
    let install_dir = dir.path().join("app");
    extract_version(
        &zip_bytes(&[("commits-app.exe", b"first build")]),
        &install_dir,
        "0.0.0-dev",
    )
    .unwrap();

    let second = extract_version(
        &zip_bytes(&[("commits-app.exe", b"second build")]),
        &install_dir,
        "0.0.0-dev",
    )
    .unwrap();

    assert_ne!(second, install_dir.join("0.0.0-dev"));
    assert!(second
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("0.0.0-dev~"));
    assert_eq!(
        fs::read(install_dir.join("0.0.0-dev/commits-app.exe")).unwrap(),
        b"first build"
    );
    assert_eq!(
        fs::read(second.join("commits-app.exe")).unwrap(),
        b"second build"
    );
}

#[test]
fn extract_version_prunes_versions_beyond_the_current_and_previous() {
    let dir = tempfile::tempdir().unwrap();
    let install_dir = dir.path().join("app");
    extract_version(&zip_bytes(&[("a", b"1")]), &install_dir, "1.0.0").unwrap();
    extract_version(&zip_bytes(&[("a", b"2")]), &install_dir, "1.1.0").unwrap();

    extract_version(&zip_bytes(&[("a", b"3")]), &install_dir, "1.2.0").unwrap();

    assert!(!install_dir.join("1.0.0").exists());
    assert!(install_dir.join("1.1.0").exists());
    assert!(install_dir.join("1.2.0").exists());
}

#[test]
fn extract_version_refuses_an_unsafe_zip_entry_path() {
    let dir = tempfile::tempdir().unwrap();
    let install_dir = dir.path().join("app");
    let archive = zip_bytes(&[("../escape.txt", b"bad")]);

    assert!(extract_version(&archive, &install_dir, "1.0.0").is_err());
}

#[test]
fn copy_version_from_dir_copies_the_running_directory_into_a_version_folder() {
    let dir = tempfile::tempdir().unwrap();
    let source_dir = dir.path().join("dev-build");
    let install_dir = dir.path().join("app");
    fs::create_dir_all(source_dir.join("components")).unwrap();
    fs::write(source_dir.join(identity().launcher_exe()), b"the launcher").unwrap();
    fs::write(source_dir.join("commits-app.exe"), b"the app").unwrap();
    fs::write(source_dir.join("components/commits.wasm"), b"component").unwrap();

    let version_dir =
        copy_version_from_dir(&identity(), &source_dir, &install_dir, "1.0.0").unwrap();

    assert_eq!(
        fs::read(version_dir.join("commits-app.exe")).unwrap(),
        b"the app"
    );
    assert!(!version_dir.join(identity().launcher_exe()).exists());
    assert_eq!(
        fs::read(version_dir.join("components/commits.wasm")).unwrap(),
        b"component"
    );
    assert!(!install_dir.join("components").exists());
}

#[test]
fn copy_version_from_dir_skips_webview2_profiles_state_and_logs() {
    let dir = tempfile::tempdir().unwrap();
    let source_dir = dir.path().join("dev-build");
    let install_dir = dir.path().join("app");
    fs::create_dir_all(source_dir.join("commits-app.exe.WebView2/EBWebView")).unwrap();
    fs::write(
        source_dir.join("commits-app.exe.WebView2/EBWebView/lock"),
        b"locked",
    )
    .unwrap();
    fs::create_dir_all(source_dir.join("state")).unwrap();
    fs::write(source_dir.join("state/save.bin"), b"save state").unwrap();
    fs::write(source_dir.join("commits.log"), b"log").unwrap();
    fs::write(source_dir.join("commits-app.exe"), b"the app").unwrap();

    let version_dir =
        copy_version_from_dir(&identity(), &source_dir, &install_dir, "1.0.0").unwrap();

    assert!(!version_dir.join("commits-app.exe.WebView2").exists());
    assert!(!version_dir.join("state").exists());
    assert!(!version_dir.join("commits.log").exists());
    assert_eq!(
        fs::read(version_dir.join("commits-app.exe")).unwrap(),
        b"the app"
    );
}

#[test]
fn copy_version_from_dir_disambiguates_a_version_string_collision() {
    let dir = tempfile::tempdir().unwrap();
    let install_dir = dir.path().join("app");
    let first_source = dir.path().join("first");
    fs::create_dir_all(&first_source).unwrap();
    fs::write(first_source.join("commits-app.exe"), b"first build").unwrap();
    copy_version_from_dir(&identity(), &first_source, &install_dir, "0.0.0-dev").unwrap();

    let second_source = dir.path().join("second");
    fs::create_dir_all(&second_source).unwrap();
    fs::write(
        second_source.join("commits-app.exe"),
        b"second build, different size",
    )
    .unwrap();
    let second =
        copy_version_from_dir(&identity(), &second_source, &install_dir, "0.0.0-dev").unwrap();

    assert_ne!(second, install_dir.join("0.0.0-dev"));
    assert_eq!(
        fs::read(install_dir.join("0.0.0-dev/commits-app.exe")).unwrap(),
        b"first build"
    );
    assert_eq!(
        fs::read(second.join("commits-app.exe")).unwrap(),
        b"second build, different size"
    );
}

#[test]
fn install_fresh_places_the_launcher_at_the_root_and_the_rest_in_a_version_folder() {
    let dir = tempfile::tempdir().unwrap();
    let source_dir = dir.path().join("dev-build");
    let install_dir = dir.path().join("app");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join(identity().launcher_exe()), b"the launcher").unwrap();
    fs::write(source_dir.join("commits-app.exe"), b"the app").unwrap();

    install_fresh(&identity(), &source_dir, &install_dir, "1.0.0").unwrap();

    assert_eq!(
        fs::read(install_dir.join(identity().launcher_exe())).unwrap(),
        b"the launcher"
    );
    assert_eq!(
        fs::read(install_dir.join("1.0.0/commits-app.exe")).unwrap(),
        b"the app"
    );
}

#[test]
fn install_fresh_finds_the_launcher_one_level_up_from_a_version_folder_source() {
    // running_directory() often resolves to a version folder now (e.g.
    // running straight out of dist/app/<version>/), where the launcher
    // is a sibling of the version folder, not inside it.
    let dir = tempfile::tempdir().unwrap();
    let build_root = dir.path().join("dist-app");
    let source_dir = build_root.join("0.3.0");
    let install_dir = dir.path().join("app");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(build_root.join(identity().launcher_exe()), b"the launcher").unwrap();
    fs::write(source_dir.join("commits-app.exe"), b"the app").unwrap();

    install_fresh(&identity(), &source_dir, &install_dir, "1.0.0").unwrap();

    assert_eq!(
        fs::read(install_dir.join(identity().launcher_exe())).unwrap(),
        b"the launcher"
    );
    assert_eq!(
        fs::read(install_dir.join("1.0.0/commits-app.exe")).unwrap(),
        b"the app"
    );
}

#[test]
fn install_fresh_fails_without_a_launcher_in_source_dir_or_its_parent() {
    let dir = tempfile::tempdir().unwrap();
    let source_dir = dir.path().join("dev-build");
    let install_dir = dir.path().join("app");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("commits-app.exe"), b"the app").unwrap();

    assert!(install_fresh(&identity(), &source_dir, &install_dir, "1.0.0").is_err());
}

/// An install dir holding one version folder, with `payload` as the
/// launcher that shipped inside it and `entry` as the entry point
/// already installed. Either can be absent.
fn install_with(
    payload: Option<&[u8]>,
    entry: Option<&[u8]>,
) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let install_dir = dir.path().join("app");
    let version_dir = install_dir.join("1.0.0");
    fs::create_dir_all(&version_dir).unwrap();
    if let Some(bytes) = payload {
        fs::write(version_dir.join(identity().launcher_payload_exe()), bytes).unwrap();
    }
    if let Some(bytes) = entry {
        fs::write(install_dir.join(identity().launcher_exe()), bytes).unwrap();
    }
    (dir, install_dir, version_dir)
}

#[test]
fn refresh_launcher_installs_a_shipped_launcher_that_differs() {
    let (_dir, install_dir, version_dir) =
        install_with(Some(b"new launcher"), Some(b"old launcher"));

    let replaced = refresh_launcher(&identity(), &install_dir, &version_dir).unwrap();

    assert!(replaced);
    assert_eq!(
        fs::read(install_dir.join(identity().launcher_exe())).unwrap(),
        b"new launcher"
    );
}

#[test]
fn refresh_launcher_moves_the_outgoing_entry_point_aside_rather_than_overwriting_it() {
    // The launcher may still be executing while this runs, and Windows
    // refuses to overwrite a running image but allows renaming one.
    // Whatever is executing the old bytes has to keep finding them.
    let (_dir, install_dir, version_dir) =
        install_with(Some(b"new launcher"), Some(b"old launcher"));

    refresh_launcher(&identity(), &install_dir, &version_dir).unwrap();

    let superseded = install_dir.join(format!(
        "{}{}",
        identity().launcher_exe(),
        SUPERSEDED_SUFFIX
    ));
    assert_eq!(fs::read(superseded).unwrap(), b"old launcher");
}

#[test]
fn refresh_launcher_sweeps_a_leftover_from_the_previous_replacement() {
    let (_dir, install_dir, version_dir) = install_with(Some(b"same"), Some(b"same"));
    let superseded = install_dir.join(format!(
        "{}{}",
        identity().launcher_exe(),
        SUPERSEDED_SUFFIX
    ));
    fs::write(&superseded, b"two versions ago").unwrap();

    refresh_launcher(&identity(), &install_dir, &version_dir).unwrap();

    assert!(
        !superseded.exists(),
        "the leftover should be gone once nothing executes it"
    );
}

#[test]
fn refresh_launcher_leaves_an_identical_entry_point_untouched() {
    let (_dir, install_dir, version_dir) = install_with(Some(b"same bytes"), Some(b"same bytes"));

    let replaced = refresh_launcher(&identity(), &install_dir, &version_dir).unwrap();

    assert!(!replaced);
    let superseded = install_dir.join(format!(
        "{}{}",
        identity().launcher_exe(),
        SUPERSEDED_SUFFIX
    ));
    assert!(
        !superseded.exists(),
        "an unchanged launcher should not be moved aside"
    );
}

#[test]
fn refresh_launcher_does_nothing_for_a_version_folder_carrying_no_launcher() {
    // Every version folder installed before launchers travelled with a
    // payload looks like this, and none of them is an error.
    let (_dir, install_dir, version_dir) = install_with(None, Some(b"the only launcher"));

    let replaced = refresh_launcher(&identity(), &install_dir, &version_dir).unwrap();

    assert!(!replaced);
    assert_eq!(
        fs::read(install_dir.join(identity().launcher_exe())).unwrap(),
        b"the only launcher"
    );
}

#[test]
fn refresh_launcher_installs_one_where_no_entry_point_exists_yet() {
    let (_dir, install_dir, version_dir) = install_with(Some(b"first launcher"), None);

    let replaced = refresh_launcher(&identity(), &install_dir, &version_dir).unwrap();

    assert!(replaced);
    assert_eq!(
        fs::read(install_dir.join(identity().launcher_exe())).unwrap(),
        b"first launcher"
    );
}

#[test]
fn refresh_launcher_never_ships_the_payload_under_the_entry_points_name() {
    // The payload has to survive in the version folder under its own
    // name: pruning aside, it is what a later start reinstalls from.
    let (_dir, install_dir, version_dir) =
        install_with(Some(b"new launcher"), Some(b"old launcher"));

    refresh_launcher(&identity(), &install_dir, &version_dir).unwrap();

    assert!(version_dir.join(identity().launcher_payload_exe()).exists());
    assert!(!version_dir.join(identity().launcher_exe()).exists());
}

#[test]
fn remove_version_dir_deletes_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    let version_dir = dir.path().join("1.0.0");
    fs::create_dir_all(&version_dir).unwrap();
    fs::write(version_dir.join("commits-app.exe"), b"bad build").unwrap();

    remove_version_dir(&version_dir).unwrap();

    assert!(!version_dir.exists());
}

#[test]
fn remove_version_dir_succeeds_when_nothing_is_there() {
    let dir = tempfile::tempdir().unwrap();
    let version_dir = dir.path().join("1.0.0");

    assert!(remove_version_dir(&version_dir).is_ok());
}
