//! Checks every package under `agent-plugins/` against Agent Plugins 1.0.0
//! (<https://agent-plugins.org/specification>) and its skills against the
//! Agent Skills frontmatter rules, offline: a client may not fetch a schema
//! while loading, so the rules are restated here rather than downloaded.
//!
//! The workspace tests run it, so a plugin no client would load fails here.

use std::path::Path;

use serde_json::{Map, Value};

use crate::Result;

/// Where the plugins live, relative to the repository root.
pub const DIR: &str = "agent-plugins";

const PLUGIN_SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
const MCP_SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";
/// `plugin.json`'s schema is closed to these keys.
const PLUGIN_KEYS: [&str; 10] = [
    "$schema",
    "name",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "extensions",
];

pub fn run(args: &[String]) -> Result {
    if !args.is_empty() {
        return Err("usage: cargo xtask plugins".to_string());
    }
    check(&crate::process::repo_root())?;
    println!("Every plugin in {DIR}/ conforms to Agent Plugins 1.0.0.");
    Ok(())
}

/// Fails with every problem in every plugin under `root`.
pub fn check(root: &Path) -> Result {
    let dir = root.join(DIR);
    let entries =
        std::fs::read_dir(&dir).map_err(|error| format!("reading {}: {error}", dir.display()))?;
    let mut problems = Vec::new();
    let mut plugins = 0;
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        plugins += 1;
        let name = entry.file_name().to_string_lossy().into_owned();
        let mut report = |problem: String| problems.push(format!("{DIR}/{name}: {problem}"));
        check_plugin(&path, &name, &mut report);
    }
    if plugins == 0 {
        problems.push(format!("{DIR}/ holds no plugin"));
    }
    match problems.is_empty() {
        true => Ok(()),
        false => Err(format!(
            "agent plugins break Agent Plugins 1.0.0:\n  {}",
            problems.join("\n  ")
        )),
    }
}

fn check_plugin(path: &Path, directory: &str, report: &mut impl FnMut(String)) {
    match read_json(&path.join("plugin.json")) {
        Ok(manifest) => check_manifest(&manifest, directory, report),
        Err(problem) => report(problem),
    }
    if path.join("mcp.json").exists() {
        match read_json(&path.join("mcp.json")) {
            Ok(mcp) => check_mcp(&mcp, report),
            Err(problem) => report(problem),
        }
    }
    let skills = path.join("skills");
    for skill in std::fs::read_dir(&skills).into_iter().flatten().flatten() {
        if skill.path().is_dir() {
            let name = skill.file_name().to_string_lossy().into_owned();
            check_skill(&skill.path(), &name, report);
        }
    }
}

fn check_manifest(manifest: &Value, directory: &str, report: &mut impl FnMut(String)) {
    let Some(object) = manifest.as_object() else {
        return report("plugin.json is not an object".into());
    };
    for key in object
        .keys()
        .filter(|key| !PLUGIN_KEYS.contains(&key.as_str()))
    {
        report(format!("plugin.json has unknown key '{key}'"));
    }
    if object.get("$schema").and_then(Value::as_str) != Some(PLUGIN_SCHEMA) {
        report(format!("plugin.json $schema must be {PLUGIN_SCHEMA}"));
    }
    match object.get("name").and_then(Value::as_str) {
        Some(name) if !is_plugin_name(name) => {
            report(format!("'{name}' is not a valid plugin name"))
        }
        Some(name) if name != directory => report(format!(
            "plugin.json names '{name}', but its directory is '{directory}'"
        )),
        Some(_) => {}
        None => report("plugin.json has no name".into()),
    }
    for key in [
        "version",
        "description",
        "homepage",
        "repository",
        "license",
    ] {
        if object.get(key).is_some_and(|value| !value.is_string()) {
            report(format!("plugin.json {key} must be a string"));
        }
    }
    if let Some(author) = object.get("author") {
        let valid = author.as_object().is_some_and(|author| {
            author.iter().all(|(key, value)| {
                ["name", "email", "url"].contains(&key.as_str()) && value.is_string()
            })
        });
        if !valid {
            report("plugin.json author takes only string name, email, and url".into());
        }
    }
    if let Some(keywords) = object.get("keywords") {
        if !keywords
            .as_array()
            .is_some_and(|k| k.iter().all(Value::is_string))
        {
            report("plugin.json keywords must be strings".into());
        }
    }
}

fn check_mcp(mcp: &Value, report: &mut impl FnMut(String)) {
    let Some(object) = mcp.as_object() else {
        return report("mcp.json is not an object".into());
    };
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    if keys != ["$schema", "mcpServers"] {
        report("mcp.json takes exactly $schema and mcpServers".into());
    }
    if object.get("$schema").and_then(Value::as_str) != Some(MCP_SCHEMA) {
        report(format!("mcp.json $schema must be {MCP_SCHEMA}"));
    }
    let servers = object.get("mcpServers").and_then(Value::as_object);
    for (name, server) in servers.into_iter().flatten() {
        if let Err(problem) = check_server(server) {
            report(format!("mcp.json server '{name}': {problem}"));
        }
    }
}

/// One server entry, which must match exactly one transport's fields.
fn check_server(server: &Value) -> std::result::Result<(), String> {
    let server = server.as_object().ok_or("not an object")?;
    let allowed: &[&str] = match server.get("type").and_then(Value::as_str) {
        Some("stdio") => &["type", "command", "args", "env", "cwd"],
        Some("streamable-http") | Some("sse") => &["type", "url", "headers"],
        _ => return Err("type must be stdio, streamable-http, or sse".into()),
    };
    if let Some(key) = server.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("unknown field '{key}'"));
    }
    if server["type"] == "stdio" {
        let command = server.get("command").and_then(Value::as_str).unwrap_or("");
        let one_token = !command.is_empty() && !command.contains(char::is_whitespace);
        let relative_or_bare = command.starts_with("./") || !command.contains(['/', '\\']);
        if !(one_token && relative_or_bare) {
            return Err(format!(
                "command '{command}' must be one bare executable name or a ./ path"
            ));
        }
        if server
            .get("args")
            .is_some_and(|a| !a.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
        {
            return Err("args must be strings".into());
        }
        if server.get("env").is_some_and(|e| !string_map(e)) {
            return Err("env must map strings to strings".into());
        }
    } else if !server
        .get("url")
        .and_then(Value::as_str)
        .is_some_and(|url| url.starts_with("http"))
    {
        return Err("url must be an absolute http(s) URL".into());
    }
    Ok(())
}

/// A skill directory: `SKILL.md` whose frontmatter names the directory and
/// describes when to use it, as the Agent Skills specification asks.
fn check_skill(path: &Path, directory: &str, report: &mut impl FnMut(String)) {
    let file = path.join("SKILL.md");
    let Ok(text) = std::fs::read_to_string(&file) else {
        return report(format!("skills/{directory} has no SKILL.md"));
    };
    let text = text.replace("\r\n", "\n");
    let Some(front) = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map(|(front, _)| front)
    else {
        return report(format!("skills/{directory}/SKILL.md has no frontmatter"));
    };
    let field = |key: &str| {
        front
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .map(str::trim)
    };
    match field("name") {
        Some(name) if name == directory && is_skill_name(name) => {}
        other => report(format!(
            "skills/{directory}/SKILL.md name is {other:?}; it must be '{directory}', lowercase letters, digits, and single hyphens"
        )),
    }
    match field("description") {
        Some(description) if !description.is_empty() && description.chars().count() <= 1024 => {}
        _ => report(format!(
            "skills/{directory}/SKILL.md needs a description of at most 1024 characters"
        )),
    }
}

/// 1–64 of `a-z`, `0-9`, `-`, `.`, starting and ending alphanumeric, with no
/// `--` or `..`.
fn is_plugin_name(name: &str) -> bool {
    let alnum = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    (1..=64).contains(&name.len())
        && name.chars().all(|c| alnum(c) || c == '-' || c == '.')
        && name.starts_with(alnum)
        && name.ends_with(alnum)
        && !name.contains("--")
        && !name.contains("..")
}

/// 1–64 of `a-z`, `0-9`, and single hyphens, not at either end.
fn is_skill_name(name: &str) -> bool {
    let alnum = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    (1..=64).contains(&name.len())
        && name.chars().all(|c| alnum(c) || c == '-')
        && name.starts_with(alnum)
        && name.ends_with(alnum)
        && !name.contains("--")
}

fn string_map(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|map: &Map<String, Value>| map.values().all(Value::is_string))
}

fn read_json(path: &Path) -> std::result::Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("parsing {}: {error}", path.display()))
}

#[cfg(test)]
mod tests;
