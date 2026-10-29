use serde_json::json;

use super::*;
use crate::process::repo_root;

#[test]
fn every_committed_plugin_conforms() {
    check(&repo_root()).unwrap_or_else(|error| panic!("{error}"));
}

fn manifest_problems(manifest: Value, directory: &str) -> Vec<String> {
    let mut problems = Vec::new();
    check_manifest(&manifest, directory, &mut |problem| problems.push(problem));
    problems
}

#[test]
fn a_manifest_needs_the_schema_and_a_name_matching_its_directory() {
    let good = json!({ "$schema": PLUGIN_SCHEMA, "name": "bones-user" });
    assert!(manifest_problems(good, "bones-user").is_empty());
    let problems = manifest_problems(json!({ "name": "Bones" }), "bones-user");
    assert!(
        problems.iter().any(|p| p.contains("$schema")),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("not a valid plugin name")),
        "{problems:?}"
    );
}

#[test]
fn the_manifest_schema_is_closed() {
    let manifest = json!({ "$schema": PLUGIN_SCHEMA, "name": "a", "commands": [] });
    let problems = manifest_problems(manifest, "a");
    assert!(
        problems.iter().any(|p| p.contains("'commands'")),
        "{problems:?}"
    );
}

#[test]
fn a_stdio_command_is_one_bare_name_or_a_relative_path() {
    assert!(
        check_server(&json!({ "type": "stdio", "command": "bones-cli", "args": ["mcp"] })).is_ok()
    );
    assert!(check_server(&json!({ "type": "stdio", "command": "./bin/tool" })).is_ok());
    assert!(check_server(&json!({ "type": "stdio", "command": "/usr/bin/tool" })).is_err());
    assert!(check_server(&json!({ "type": "stdio", "command": "tool --flag" })).is_err());
    assert!(check_server(&json!({ "type": "stdio", "command": "tool", "url": "x" })).is_err());
}

#[test]
fn names_follow_the_specifications() {
    assert!(is_plugin_name("acme.tools"));
    assert!(!is_plugin_name("a--b"));
    assert!(!is_plugin_name("-a"));
    assert!(is_skill_name("bones-dev"));
    assert!(!is_skill_name("bones.dev"));
}
