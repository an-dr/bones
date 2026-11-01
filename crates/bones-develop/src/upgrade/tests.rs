use super::*;

#[test]
fn engine_and_abi_pins_move_and_other_lines_stay() {
    let manifest = "[dependencies]\n\
        bones-wasm-sdk = { git = \"https://x\", tag = \"abi-v1.3.0\" }\n\
        bones-test = { git = \"https://x\", tag = \"v3.1.0\" }\n\
        other = { git = \"https://y\", branch = \"main\" }\n\
        serde = \"1\"\n";
    let updated = repin(manifest, "v3.2.0", "1.4.0");
    assert_eq!(
        updated,
        "[dependencies]\n\
         bones-wasm-sdk = { git = \"https://x\", tag = \"abi-v1.4.0\" }\n\
         bones-test = { git = \"https://x\", tag = \"v3.2.0\" }\n\
         other = { git = \"https://y\", branch = \"main\" }\n\
         serde = \"1\"\n"
    );
}

#[test]
fn a_manifest_without_pins_is_unchanged() {
    let manifest = "[package]\nname = \"pong\"\n";
    assert_eq!(repin(manifest, "v3.2.0", "1.4.0"), manifest);
}

#[test]
fn another_crates_git_tag_is_left_alone() {
    let manifest = "other = { git = \"https://y\", tag = \"v1.0.0\" }\n";
    assert_eq!(repin(manifest, "v3.2.0", "1.4.0"), manifest);
}

#[test]
fn only_changed_files_are_rewritten_and_reported() {
    let root = std::env::temp_dir().join("bones-develop-agents-update");
    let _ = std::fs::remove_dir_all(&root);
    let rules = root.join("checkout").join(RULES_PATH);
    std::fs::create_dir_all(rules.parent().unwrap()).unwrap();
    std::fs::write(&rules, "1. A new rule.\n").unwrap();
    let source = Source::at(&root.join("checkout"));
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();

    let changed = update(&project, &source, "bones-develop").unwrap();
    assert_eq!(changed, ["AGENTS.md", ".mcp.json"]);
    let agents = std::fs::read_to_string(project.join("AGENTS.md")).unwrap();
    assert!(agents.ends_with("1. A new rule.\n"), "{agents}");

    assert!(update(&project, &source, "bones-develop")
        .unwrap()
        .is_empty());
    assert_eq!(
        update(&project, &source, "C:/other/bones-develop.exe").unwrap(),
        [".mcp.json"]
    );
    std::fs::remove_dir_all(root).ok();
}
