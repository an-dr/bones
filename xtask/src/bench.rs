//! `cargo xtask bench`: the agent benchmark's runner (tests/agent/README.md).
//!
//! - `prepare` lays out a clean workspace: the built distribution and the
//!   task's prompt, plus a starter project for a task that has one.
//! - `score` copies the task's hidden acceptance tests into the agent's
//!   project and runs them, printing one JSON line.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::files::{copy_dir, copy_file, create_dir, exe_name, write_file};
use crate::process::{cargo, repo_root};
use crate::Result;

const USAGE: &str = "\
usage: cargo xtask bench list
       cargo xtask bench prepare <task> <workspace> [--local]
       cargo xtask bench score <task> <workspace>";

/// Tasks that start from a broken project, and that project's name.
const STARTERS: [(&str, &str); 1] = [("repair", "orbit")];

pub fn run(args: &[String]) -> Result {
    let root = repo_root();
    match args.first().map(String::as_str) {
        Some("list") => {
            for task in tasks(&root)? {
                println!("{task}");
            }
            Ok(())
        }
        Some("prepare") => {
            let (task, workspace) = task_and_workspace(&root, &args[1..])?;
            let local = args.iter().any(|arg| arg == "--local");
            prepare(&root, &task, &workspace, local)
        }
        Some("score") => {
            let (task, workspace) = task_and_workspace(&root, &args[1..])?;
            score(&root, &task, &workspace)
        }
        _ => Err(USAGE.to_string()),
    }
}

fn tasks(root: &Path) -> Result<Vec<String>> {
    let dir = root.join("tests/agent/tasks");
    let mut tasks: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|error| format!("reading {}: {error}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("TASK.md").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    tasks.sort();
    Ok(tasks)
}

fn task_and_workspace(root: &Path, args: &[String]) -> Result<(String, PathBuf)> {
    let (Some(task), Some(workspace)) = (args.first(), args.get(1)) else {
        return Err(USAGE.to_string());
    };
    if !tasks(root)?.contains(task) {
        return Err(format!(
            "no task named '{task}'; cargo xtask bench list names them"
        ));
    }
    let workspace = std::path::absolute(workspace).map_err(|error| error.to_string())?;
    Ok((task.clone(), workspace))
}

fn prepare(root: &Path, task: &str, workspace: &Path, local: bool) -> Result {
    let dist = root.join("dist");
    if !dist.join(exe_name("bones-cli")).is_file() {
        return Err("no dist/ with bones-cli; run cargo xtask dist --no-archive first".to_string());
    }
    if workspace.exists()
        && std::fs::read_dir(workspace)
            .map(|mut d| d.next().is_some())
            .unwrap_or(true)
    {
        return Err(format!(
            "{} is not empty; give a new folder",
            workspace.display()
        ));
    }
    let engine = workspace.join("engine");
    create_dir(workspace)?;
    copy_dir(&dist, &engine)?;

    let bones_path = match local {
        true => format!(" --bones-path {}", slashes(root)),
        false => String::new(),
    };
    let prompt = std::fs::read_to_string(root.join("tests/agent/tasks").join(task).join("TASK.md"))
        .map_err(|error| format!("reading the task: {error}"))?
        .replace("{{engine}}", &slashes(&engine))
        .replace("{{bones_path}}", &bones_path);
    write_file(&workspace.join("TASK.md"), &prompt)?;

    if let Some((_, name)) = STARTERS.iter().find(|(starter, _)| *starter == task) {
        let mut new = Command::new(engine.join(exe_name("bones-cli")));
        new.current_dir(workspace)
            .args(["new", name, "--kind", "game"]);
        if local {
            new.arg("--bones-path").arg(root);
        }
        let status = new
            .status()
            .map_err(|error| format!("running bones-cli: {error}"))?;
        if !status.success() {
            return Err("bones-cli new failed for the starter".to_string());
        }
        copy_file(
            &root
                .join("tests/agent/tasks")
                .join(task)
                .join("starter/lib.rs"),
            &workspace.join(name).join("extension/src/lib.rs"),
        )?;
    }
    println!(
        "Prepared {}: give an agent TASK.md and nothing else.",
        workspace.display()
    );
    Ok(())
}

fn score(root: &Path, task: &str, workspace: &Path) -> Result {
    let project = find_project(workspace).ok_or_else(|| {
        format!(
            "no project (bones.toml beside checks/) under {}",
            workspace.display()
        )
    })?;
    refresh_sources(&project.join("extension/src"))?;
    let acceptance = root.join("tests/agent/tasks").join(task).join("acceptance");
    let mut targets = Vec::new();
    for entry in std::fs::read_dir(&acceptance).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let target = format!("acceptance_{stem}");
        copy_file(
            &path,
            &project.join("checks/tests").join(format!("{target}.rs")),
        )?;
        targets.push(target);
    }
    let mut command = cargo(&project, &["test", "--quiet"]);
    for target in &targets {
        command.args(["--test", target]);
    }
    let output = command
        .output()
        .map_err(|error| format!("running cargo: {error}"))?;
    let text = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    let (passed, failed) = counts(&text);
    println!(
        "{}",
        serde_json::json!({
            "task": task,
            "project": project.display().to_string(),
            "passed": passed,
            "failed": failed,
            "built": output.status.success() || passed + failed > 0,
        })
    );
    if !output.status.success() {
        eprintln!("{text}");
    }
    Ok(())
}

/// Marks every extension source as changed now, so cargo rebuilds the
/// component rather than reuse a same-named build a shared target directory
/// may hold from another workspace.
fn refresh_sources(dir: &Path) -> Result {
    let now = std::time::SystemTime::now();
    for entry in
        std::fs::read_dir(dir).map_err(|error| format!("reading {}: {error}", dir.display()))?
    {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            refresh_sources(&path)?;
        } else {
            std::fs::File::options()
                .write(true)
                .open(&path)
                .and_then(|file| file.set_modified(now))
                .map_err(|error| format!("touching {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

/// The folder holding `bones.toml` beside `checks/`, at most two levels down.
fn find_project(workspace: &Path) -> Option<PathBuf> {
    let is_project = |dir: &Path| dir.join("bones.toml").is_file() && dir.join("checks").is_dir();
    let children = |dir: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_dir() && !path.ends_with("engine"))
            .collect()
    };
    children(workspace)
        .into_iter()
        .flat_map(|child| {
            let mut found = vec![child.clone()];
            found.extend(children(&child));
            found
        })
        .find(|dir| is_project(dir))
}

/// Passed and failed counts summed over cargo's `test result:` lines.
fn counts(output: &str) -> (u64, u64) {
    let number = |line: &str, word: &str| -> u64 {
        line.split(';')
            .find(|part| part.trim_end().ends_with(word))
            .and_then(|part| part.split_whitespace().rev().nth(1)?.parse().ok())
            .unwrap_or(0)
    };
    output
        .lines()
        .filter(|line| line.contains("test result:"))
        .fold((0, 0), |(passed, failed), line| {
            (
                passed + number(line, "passed"),
                failed + number(line, "failed"),
            )
        })
}

fn slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests;
