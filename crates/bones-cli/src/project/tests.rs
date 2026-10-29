use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bones-cli-test-{name}"));
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
    assert!(error.contains("bones-cli new"), "{error}");
}
