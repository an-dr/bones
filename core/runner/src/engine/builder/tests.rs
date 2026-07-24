use super::*;

#[test]
fn tick_hz_defaults_to_60_and_is_overridable() {
    assert_eq!(Engine::new().tick_hz, DEFAULT_TICK_HZ);
    assert_eq!(Engine::new().tick_hz(30.0).tick_hz, 30.0);
}

#[test]
fn an_absolute_path_passes_through_resolve_relative_to_exe_unchanged() {
    let absolute = if cfg!(windows) {
        PathBuf::from(r"C:\somewhere\saves")
    } else {
        PathBuf::from("/somewhere/saves")
    };

    assert_eq!(resolve_relative_to_exe(absolute.clone()), absolute);
}

#[test]
fn a_relative_path_resolves_against_the_test_binarys_own_directory() {
    // cargo test's own executable always has a determinable path, so this
    // exercises the real `exe_dir()` lookup end to end.
    let resolved = resolve_relative_to_exe(PathBuf::from("saves"));

    assert!(resolved.is_absolute());
    assert!(resolved.ends_with("saves"));
}
