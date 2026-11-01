use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bones-develop-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("extension/src")).unwrap();
    std::fs::write(dir.join("bones.toml"), "").unwrap();
    std::fs::write(
        dir.join("extension/Cargo.toml"),
        "[package]\nname = \"pong-game\"\n\n[dependencies]\nbones-wasm-sdk = { git = \"x\", tag = \"abi-v1.3.0\" }\n",
    )
    .unwrap();
    dir
}

#[test]
fn a_project_is_found_from_a_directory_inside_it() {
    let root = scratch("find");
    let project = Project::find_from(&root.join("extension/src")).unwrap();
    assert_eq!(project.name, "pong-game");
    assert_eq!(project.pinned_abi().as_deref(), Some("1.3.0"));
    assert!(project
        .installed_wasm()
        .ends_with(".bones/extensions/pong_game.wasm"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn outside_a_project_says_how_to_make_one() {
    let error = Project::find_from(Path::new("/")).unwrap_err();
    assert!(error.contains("bones-develop new"), "{error}");
}

#[test]
fn the_engine_tag_comes_from_the_checks_manifest_not_the_abi_pin() {
    let root = scratch("pinned-engine");
    let project = Project::find_from(&root).unwrap();
    assert_eq!(project.pinned_engine(), None, "abi-v is not an engine tag");
    std::fs::create_dir_all(root.join("checks")).unwrap();
    std::fs::write(
        root.join("checks/Cargo.toml"),
        "[dev-dependencies]\nbones-test = { git = \"x\", tag = \"v3.1.0\" }\n",
    )
    .unwrap();
    assert_eq!(project.pinned_engine().as_deref(), Some("v3.1.0"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn web_panels_are_read_from_bones_toml() {
    let root = scratch("web");
    let project = Project::find_from(&root).unwrap();
    assert!(!project.uses_web());
    std::fs::write(root.join("bones.toml"), "# web = true\nweb = false\n").unwrap();
    assert!(!project.uses_web(), "a comment or false is not web");
    std::fs::write(root.join("bones.toml"), "web = true # panels\n").unwrap();
    assert!(project.uses_web());
    std::fs::remove_dir_all(root).ok();
}
