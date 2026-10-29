use super::*;

#[test]
fn the_committed_reference_matches_its_sources() {
    let root = repo_root();
    let generated = generate(&root).unwrap_or_else(|error| panic!("{error}"));
    let committed = std::fs::read_to_string(root.join(OUTPUT)).unwrap_or_default();
    assert!(
        committed.replace("\r\n", "\n") == generated,
        "{OUTPUT} is stale; run `cargo xtask plugin-docs`"
    );
}

#[test]
fn the_skill_points_at_the_reference() {
    let skill =
        std::fs::read_to_string(repo_root().join("agent-plugins/bones-user/skills/bones/SKILL.md"))
            .unwrap();
    assert!(
        skill.contains("references/docs.md"),
        "SKILL.md does not link references/docs.md"
    );
}
