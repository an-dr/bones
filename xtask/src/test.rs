//! `cargo xtask test`: builds every extension fixture the test suite needs,
//! then runs the gates and the whole test matrix. One command from a clean
//! clone to a release-green tree.
//!
//! This is where formatting, lints, and the optional-feature suite are
//! enforced, locally and in CI (.github/workflows/ci.yml): a gate that lives
//! only in a reviewer's habits is a gate that does not exist.

use std::path::Path;

use crate::metadata::{cargo_metadata, packages};
use crate::prereq::{self, Check};
use crate::process::{cargo, read_output, repo_root, run_step};
use crate::Result;

/// Excluded from the root workspace (see its Cargo.toml) because they must
/// also compile for wasm32-wasip2 guests, so every check below runs them
/// separately, from inside their own directories.
const STANDALONE_CRATES: [&str; 3] = [
    "crates/bones-messages",
    "crates/bones-wasm-sdk",
    // An example, but a wire contract too: it is the only place a custom
    // message vocabulary is defined, and it compiles without SDL, so checking
    // it costs seconds.
    "examples/embedding/custom-engine/messages",
];

/// Integration tests load these as prebuilt .wasm files by path, so they
/// must exist before cargo test runs -- a missing one fails the test rather
/// than skipping it. Each builds to its own
/// `target/wasm32-wasip2/release/<name>.wasm`.
const FIXTURES: [&str; 9] = [
    "examples/extensions/hello-demo",
    "examples/extensions/keyecho-demo",
    "examples/extensions/sprite-demo",
    "examples/extensions/runaway-demo",
    "examples/extensions/flood-demo",
    "examples/extensions/audio-demo",
    "examples/extensions/persistence-demo",
    // dashboard and metrics back the web-panel tests, which are gated behind
    // --features web on Windows. Built unconditionally: cheap, and it keeps
    // the output identical whether or not those tests run.
    "examples/extensions/dashboard-demo",
    "examples/extensions/metrics-demo",
];

pub fn run() -> Result {
    // The suite exercises everything, so it needs everything.
    prereq::require(&Check::ALL)?;
    let root = repo_root();

    for fixture in FIXTURES {
        // Only the .wasm: assembling an example's dist/ would also build the
        // engine, which the test suite does not need.
        run_step(
            &format!("Building fixture '{fixture}'"),
            &mut cargo(
                &root.join(fixture),
                &["build", "--target", "wasm32-wasip2", "--release"],
            ),
        )?;
    }

    // Deliberately not `cargo fmt --all`: that also formats local path
    // dependencies, which here means rewriting the vendored pubsub-bus
    // submodule even though the workspace excludes it.
    for member in workspace_members(&root)? {
        run_step(
            &format!("Checking formatting of {member}"),
            &mut cargo(&root, &["fmt", "-p", &member, "--", "--check"]),
        )?;
    }
    run_in_standalone_crates(&root, "Checking formatting", &["fmt", "--", "--check"])?;

    run_step(
        "Linting the workspace",
        &mut cargo(
            &root,
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        ),
    )?;
    run_in_standalone_crates(
        &root,
        "Linting",
        &[
            "clippy",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;

    // Both feature sets, because they exercise different code: the default
    // build is what a distribution ships, and --all-features is the only
    // thing that compiles and runs the optional wry web panels.
    run_step(
        "Running workspace tests (default features)",
        &mut cargo(&root, &["test", "--workspace"]),
    )?;
    run_step(
        "Running workspace tests (all features)",
        &mut cargo(&root, &["test", "--workspace", "--all-features"]),
    )?;
    check_kernel_without_wasm(&root)?;
    // --all-features so the SDK's optional game-ui module is covered too.
    run_in_standalone_crates(&root, "Running tests", &["test", "--all-features"])?;

    run_step(
        "Checking the documentation build",
        cargo(
            &root,
            &["doc", "--workspace", "--all-features", "--no-deps"],
        )
        .env("RUSTDOCFLAGS", "-D warnings"),
    )?;

    println!();
    println!("All gates and tests passed.");
    Ok(())
}

fn run_in_standalone_crates(root: &Path, description: &str, args: &[&str]) -> Result {
    for dir in STANDALONE_CRATES {
        run_step(
            &format!("{description} ({dir})"),
            &mut cargo(&root.join(dir), args),
        )?;
    }
    Ok(())
}

/// The packages the workspace owns, asked of cargo so the list stays
/// correct when a crate is added.
fn workspace_members(root: &Path) -> Result<Vec<String>> {
    let metadata = cargo_metadata(root, &["--no-deps"])?;
    let members = packages(&metadata)
        .filter_map(|package| package["name"].as_str().map(str::to_string))
        .collect();
    Ok(members)
}

/// Extensions are opt-in (ADR-040), and only a build without them proves it.
///
/// The workspace runs above always unify `wasm` on, because the `bones`
/// executable enables it, so they cannot catch a kernel that has quietly
/// started to need the extension host. This tests `bones-kernel` alone, with
/// its own default features, and checks that wasmtime is not in its tree.
fn check_kernel_without_wasm(root: &Path) -> Result {
    run_step(
        "Testing bones-kernel without the wasm feature",
        &mut cargo(root, &["test", "-p", "bones-kernel"]),
    )?;
    println!("==> Checking that bones-kernel links no WASM runtime without it...");
    let tree = read_output(&mut cargo(
        root,
        &["tree", "-p", "bones-kernel", "--edges", "normal"],
    ))
    .ok_or("cargo tree -p bones-kernel failed")?;
    if tree.lines().any(|line| line.contains("wasmtime ")) {
        return Err(
            "bones-kernel depends on wasmtime without the wasm feature; \
             only bones-module-wasm may (ADR-040)"
                .to_string(),
        );
    }
    Ok(())
}
