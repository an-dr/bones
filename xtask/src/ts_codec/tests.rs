use super::*;

#[test]
fn the_committed_module_matches_the_specification() {
    let root = repo_root();
    let generated = generate(&root).unwrap_or_else(|error| panic!("{error}"));
    let committed = std::fs::read_to_string(root.join(OUTPUT)).unwrap_or_default();
    assert!(
        committed.replace("\r\n", "\n") == generated,
        "{OUTPUT} is stale; run `cargo xtask ts-codec`"
    );
}

#[test]
fn topic_names_become_type_names() {
    assert_eq!(type_name("gfx/draw-rect"), "GfxDrawRect");
    assert_eq!(type_name("core/extensions/load"), "CoreExtensionsLoad");
}

#[test]
fn unions_and_sequences_are_left_out() {
    let generated = generate(&repo_root()).unwrap();
    assert!(generated.contains("\"input/key-down\": InputKeyDown;"));
    assert!(!generated.contains("\"game-core/entity-op\""));
    assert!(!generated.contains("\"ui/spec\""));
}
