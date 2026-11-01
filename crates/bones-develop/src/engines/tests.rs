use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bones-develop-engines-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// A repository with one commit, tagged `tag`.
fn repository(name: &str, tag: &str) -> PathBuf {
    let repo = scratch(name);
    git(&repo, &["init", "--quiet"]);
    std::fs::write(repo.join("Cargo.toml"), "[workspace]\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "--quiet", "-m", "start"]);
    git(&repo, &["tag", tag]);
    repo
}

#[test]
fn tags_that_are_not_one_folder_are_refused() {
    for tag in ["", "../v1", "v1/../x", "a/b", "a\\b", "-v1"] {
        assert!(check_tag(tag).is_err(), "{tag:?}");
    }
    for tag in ["v3.1.0", "main", "v3.2.0-rc.1"] {
        assert!(check_tag(tag).is_ok(), "{tag:?}");
    }
}

#[test]
fn versions_sort_numerically() {
    let mut tags = vec!["v3.10.0", "v3.9.1", "v3.9.0", "v10.0.0"];
    tags.sort_by_key(|tag| version_key(tag));
    assert_eq!(tags, ["v3.9.0", "v3.9.1", "v3.10.0", "v10.0.0"]);
}

#[test]
fn a_tag_is_cloned_once_and_reused() {
    let repo = repository("clone-repo", "v0.0.1");
    let home = scratch("clone-home");
    let url = repo.to_string_lossy().into_owned();
    let source = source(&home, &url, "v0.0.1").unwrap();
    assert!(source.join("Cargo.toml").is_file());
    std::fs::write(source.join("marker"), "kept").unwrap();
    assert_eq!(super::source(&home, &url, "v0.0.1").unwrap(), source);
    assert!(
        source.join("marker").is_file(),
        "a cached source is not cloned again"
    );
    assert!(
        super::source(&home, &url, "v9.9.9").is_err(),
        "an unknown tag fails"
    );
}

#[test]
fn remove_deletes_the_engines_and_the_source() {
    let home = scratch("remove-home");
    for dir in ["engines/v1.0.0", "engines/v1.0.0-web", "sources/v1.0.0"] {
        std::fs::create_dir_all(home.join(dir)).unwrap();
    }
    remove(&home, "v1.0.0").unwrap();
    assert!(!home.join("engines/v1.0.0").exists());
    assert!(!home.join("engines/v1.0.0-web").exists());
    assert!(!home.join("sources/v1.0.0").exists());
    assert!(remove(&home, "v1.0.0").is_err(), "nothing left to remove");
}

#[test]
fn an_installed_engine_is_not_rebuilt() {
    let home = scratch("installed-home");
    let dir = home.join("engines/v1.0.0-web");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(executable("bones")), "").unwrap();
    assert_eq!(
        install(&home, "v1.0.0", true).unwrap(),
        dir.join(executable("bones"))
    );
}
