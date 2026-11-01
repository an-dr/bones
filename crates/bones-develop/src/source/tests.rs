use std::path::PathBuf;

use super::*;

fn checkout(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bones-develop-source-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("crates/bones-messages")).unwrap();
    std::fs::write(
        dir.join("crates/bones-messages/Cargo.toml"),
        "[package]\nname = \"bones-messages\"\nversion = \"1.2.0\"\n",
    )
    .unwrap();
    dir
}

#[test]
fn the_abi_is_the_messages_version() {
    let root = checkout("abi");
    assert_eq!(Source::at(&root).abi().unwrap(), "1.2.0");
    assert!(Source::at(&root.join("missing")).abi().is_err());
}

#[test]
fn files_come_from_the_checkout_with_the_compiled_copy_as_fallback() {
    let root = checkout("read");
    let source = Source::at(&root);
    assert_eq!(source.read("AGENTS.md", "compiled"), "compiled");
    std::fs::write(root.join("AGENTS.md"), "from the tag").unwrap();
    assert_eq!(source.read("AGENTS.md", "compiled"), "from the tag");
}

#[test]
fn templates_are_found_under_the_old_crate_name_too() {
    let root = checkout("templates");
    let source = Source::at(&root);
    assert_eq!(source.template("game/lib.rs", "compiled"), "compiled");
    std::fs::create_dir_all(root.join("crates/bones-cli/templates/game")).unwrap();
    std::fs::write(root.join("crates/bones-cli/templates/game/lib.rs"), "old").unwrap();
    assert_eq!(source.template("game/lib.rs", "compiled"), "old");
    std::fs::create_dir_all(root.join("crates/bones-develop/templates/game")).unwrap();
    std::fs::write(
        root.join("crates/bones-develop/templates/game/lib.rs"),
        "new",
    )
    .unwrap();
    assert_eq!(source.template("game/lib.rs", "compiled"), "new");
}
