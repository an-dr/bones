use super::*;

#[test]
fn launch_probe() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !args.iter().any(|arg| arg == "--exact") {
        return;
    }
    if let Some(path) = std::env::var_os("BONES_LAUNCH_TEST_OUTPUT") {
        let path = std::path::PathBuf::from(path);
        let temporary = path.with_extension("tmp");
        std::fs::write(&temporary, args.join("\n")).unwrap();
        std::fs::rename(temporary, path).unwrap();
    }
}

#[test]
fn starts_the_selected_executable_and_forwards_remaining_arguments() {
    use std::time::{Duration, Instant};
    let dir = tempfile::tempdir().unwrap();
    let previous = dir.path().join("1.0.0");
    std::fs::create_dir(&previous).unwrap();
    std::fs::create_dir(dir.path().join("2.0.0")).unwrap();
    let name = if cfg!(windows) { "probe.exe" } else { "probe" };
    std::fs::copy(std::env::current_exe().unwrap(), previous.join(name)).unwrap();
    let output = dir.path().join("arguments.txt");
    std::env::set_var("BONES_LAUNCH_TEST_OUTPUT", &output);
    let args = [
        "--previous",
        "--exact",
        "tests::launch_probe",
        "--nocapture",
    ]
    .map(str::to_string);
    let launched = run(name, dir.path(), &args);
    std::env::remove_var("BONES_LAUNCH_TEST_OUTPUT");
    launched.unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !output.is_file() {
        assert!(
            Instant::now() < deadline,
            "the selected application never started"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        std::fs::read_to_string(output).unwrap(),
        "--exact\ntests::launch_probe\n--nocapture"
    );
    assert!(dir.path().join("2.0.0").is_dir());
}

#[test]
fn refuses_a_missing_installation() {
    let dir = tempfile::tempdir().unwrap();
    assert!(run("app.exe", dir.path(), &[])
        .unwrap_err()
        .contains("no selected version"));
}

#[test]
fn previous_selection_preserves_both_versions() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("1.0.0")).unwrap();
    std::fs::create_dir_all(dir.path().join("2.0.0")).unwrap();
    let error = run("missing.exe", dir.path(), &["--previous".into()]).unwrap_err();
    assert!(error.contains("1.0.0"), "{error}");
    assert!(dir.path().join("1.0.0").is_dir());
    assert!(dir.path().join("2.0.0").is_dir());
}

#[test]
fn application_flags_are_forwarded_instead_of_handled_by_the_launcher() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("1.0.0")).unwrap();
    for flag in ["--help", "--version", "--rollback", "--install"] {
        assert!(run("missing.exe", dir.path(), &[flag.into()])
            .unwrap_err()
            .contains("could not launch"));
        assert!(dir.path().join("1.0.0").is_dir());
    }
}
