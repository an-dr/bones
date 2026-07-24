use super::*;

#[test]
fn an_absolute_path_passes_through_unchanged_regardless_of_base() {
    let absolute = if cfg!(windows) {
        PathBuf::from(r"C:\somewhere\bones.toml")
    } else {
        PathBuf::from("/somewhere/bones.toml")
    };

    assert_eq!(
        join_relative(&absolute, Some(PathBuf::from("/exe/dir"))),
        absolute
    );
    assert_eq!(join_relative(&absolute, None), absolute);
}

#[test]
fn a_relative_path_joins_onto_the_given_base() {
    let joined = join_relative(Path::new("extensions"), Some(PathBuf::from("/exe/dir")));

    assert_eq!(joined, PathBuf::from("/exe/dir").join("extensions"));
}

#[test]
fn a_relative_path_stays_relative_with_no_base() {
    let path = Path::new("extensions");

    assert_eq!(join_relative(path, None), path);
}

#[test]
fn relative_to_exe_anchors_onto_the_test_binarys_own_directory() {
    // cargo test's own executable always has a determinable path, so this
    // exercises the real `exe_dir()` lookup end to end.
    let resolved = relative_to_exe("extensions");

    assert!(resolved.is_absolute());
    assert!(resolved.ends_with("extensions"));
}

#[test]
fn config_path_defaults_to_bones_toml_next_to_the_exe_with_no_override() {
    let resolved = resolve_config_path(None);

    assert!(resolved.is_absolute());
    assert!(resolved.ends_with("bones.toml"));
}

#[test]
fn config_path_honors_an_explicit_override() {
    let resolved = resolve_config_path(Some(PathBuf::from("/somewhere/custom.toml")));

    assert_eq!(resolved, PathBuf::from("/somewhere/custom.toml"));
}

#[test]
fn config_relative_resolves_against_the_overrides_own_directory() {
    let resolved = resolve_config_relative(
        Path::new("extensions"),
        Some(PathBuf::from("/somewhere/custom.toml")),
    );

    assert_eq!(resolved, PathBuf::from("/somewhere/extensions"));
}

#[test]
fn config_relative_falls_back_to_the_exe_dir_with_no_override() {
    let resolved = resolve_config_relative(Path::new("extensions"), None);

    assert!(resolved.is_absolute());
    assert!(resolved.ends_with("extensions"));
}
