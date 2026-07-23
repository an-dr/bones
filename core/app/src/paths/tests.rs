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
