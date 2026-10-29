//! `cargo xtask example [<name>]`: builds one example and assembles a
//! runnable dist/ inside its directory. With no name, lists the examples.
//!
//! Building only an extension's .wasm needs no xtask: each example's README
//! gives the plain `cargo build --target wasm32-wasip2 --release`.

use std::path::{Path, PathBuf};

use crate::archive::{sha256_hex, write_zip};
use crate::files::{copy_file, create_dir, exe_name, recreate_dir, write_file};
use crate::prereq::{self, Check};
use crate::process::{cargo, repo_root, run_step};
use crate::Result;

// Private to this file: one row of the example table below, never named
// outside it (docs/code-style.md's exception).
/// An extension example run by the stock `bones` engine.
struct ExtensionExample {
    /// Its directory under examples/extensions/.
    name: &'static str,
    /// Other extension examples packaged beside it.
    companions: &'static [&'static str],
    /// Whether it needs the engine built with `--features web`.
    web: bool,
}

const EXTENSION_EXAMPLES: [ExtensionExample; 12] = [
    extension("audio-demo"),
    // The dashboard page shows live data from metrics-demo.
    ExtensionExample {
        name: "dashboard-demo",
        companions: &["metrics-demo"],
        web: true,
    },
    // Floods the bus while hello-demo shows the engine staying responsive.
    ExtensionExample {
        name: "flood-demo",
        companions: &["hello-demo"],
        web: false,
    },
    extension("game-core-demo"),
    extension("hello-demo"),
    extension("keyecho-demo"),
    extension("level-demo"),
    extension("metrics-demo"),
    extension("notes-demo"),
    extension("persistence-demo"),
    extension("runaway-demo"),
    extension("sprite-demo"),
];

const fn extension(name: &'static str) -> ExtensionExample {
    ExtensionExample {
        name,
        companions: &[],
        web: false,
    }
}

/// Examples under examples/embedding/, each with its own composition root
/// and so its own build.
const EMBEDDING_EXAMPLES: [&str; 3] = ["custom-engine", "self-updating-app", "single-instance-app"];

pub fn run(args: &[String]) -> Result {
    let Some(name) = args.first() else {
        print_examples();
        return Ok(());
    };
    let root = repo_root();
    if let Some(example) = EXTENSION_EXAMPLES.iter().find(|e| e.name == name) {
        prereq::require(&[
            Check::Rustup,
            Check::RustTarget("wasm32-wasip2"),
            Check::CCompiler,
            Check::CMake,
        ])?;
        return build_extension_example(&root, example);
    }
    match name.as_str() {
        "custom-engine" => {
            prereq::require(&[
                Check::Rustup,
                Check::RustTarget("wasm32-wasip2"),
                Check::CCompiler,
                Check::CMake,
            ])?;
            build_custom_engine(&root)
        }
        "self-updating-app" => build_self_updating_app(&root),
        "single-instance-app" => {
            prereq::require(&[Check::CCompiler, Check::CMake])?;
            build_single_instance_app(&root)
        }
        _ => Err(format!(
            "no example named '{name}'; run cargo xtask example to list them"
        )),
    }
}

fn print_examples() {
    println!("usage: cargo xtask example <name>\n\nextensions:");
    for example in &EXTENSION_EXAMPLES {
        println!("  {}", example.name);
    }
    println!("\nembedding:");
    for name in EMBEDDING_EXAMPLES {
        println!("  {name}");
    }
}

/// Builds package `name` in `crate_dir` as a component and returns the path
/// of its .wasm. Every extension crate is its own workspace, so the output
/// lands in its own target/.
fn build_wasm(crate_dir: &Path, name: &str) -> Result<PathBuf> {
    println!("==> Building the {name} extension...");
    bones_build::Build::new(crate_dir).show_diagnostics().run()
}

fn build_extension_example(root: &Path, example: &ExtensionExample) -> Result {
    let extensions = root.join("examples/extensions");
    let dir = extensions.join(example.name);
    let mut wasm = vec![build_wasm(&dir, example.name)?];
    for companion in example.companions {
        wasm.push(build_wasm(&extensions.join(companion), companion)?);
    }

    let mut build = vec!["build", "-p", "bones", "--release"];
    if example.web {
        build.extend(["--features", "web"]);
    }
    run_step("Building bones", &mut cargo(root, &build))?;

    let exe = exe_name("bones");
    let dist = dir.join("dist");
    recreate_dir(&dist)?;
    create_dir(&dist.join("extensions"))?;
    copy_file(&root.join("target/release").join(&exe), &dist.join(&exe))?;
    copy_file(&dir.join("bones.toml"), &dist.join("bones.toml"))?;
    for component in &wasm {
        let file_name = component.file_name().expect("a .wasm path has a file name");
        copy_file(component, &dist.join("extensions").join(file_name))?;
    }

    println!();
    println!("Packaged: {}", dist.join(&exe).display());
    Ok(())
}

/// The executable here is *not* the shipped `bones`: it is the example's own
/// binary, which is the entire point. hello-demo runs beside the custom
/// extension so a stock and a custom extension share one engine.
fn build_custom_engine(root: &Path) -> Result {
    let dir = root.join("examples/embedding/custom-engine");
    let host_probe = build_wasm(&dir.join("extension"), "host-probe")?;
    let hello = build_wasm(&root.join("examples/extensions/hello-demo"), "hello-demo")?;
    run_step(
        "Building the custom engine",
        &mut cargo(&dir.join("engine"), &["build", "--release"]),
    )?;

    let exe = exe_name("custom-engine");
    let dist = dir.join("dist");
    recreate_dir(&dist)?;
    create_dir(&dist.join("extensions"))?;
    copy_file(
        &dir.join("engine/target/release").join(&exe),
        &dist.join(&exe),
    )?;
    copy_file(&dir.join("bones.toml"), &dist.join("bones.toml"))?;
    copy_file(&host_probe, &dist.join("extensions/host_probe.wasm"))?;
    copy_file(&hello, &dist.join("extensions/hello_demo.wasm"))?;

    println!();
    println!("Packaged: {}", dist.join(&exe).display());
    println!("Run it, and the log shows host-probe asking the native module for facts it cannot reach itself.");
    Ok(())
}

/// Builds the demo app twice -- 1.0.0 and 1.1.0 -- installs the first, and
/// publishes the second as a manifest the installed copy can find. One
/// version could only show "up to date", which proves nothing about
/// replacing a running application with a newer copy of itself.
fn build_self_updating_app(root: &Path) -> Result {
    let dir = root.join("examples/embedding/self-updating-app");
    let app = dir.join("app");
    let release = app.join("target/release");
    let (app_exe, launcher_exe, entry_exe) = (
        exe_name("demo-app"),
        exe_name("demo-launcher"),
        exe_name("demo"),
    );
    let dist = dir.join("dist");
    recreate_dir(&dist)?;

    // The newer build first, packaged as the asset the older one will find:
    // a plain zip of a version folder's contents, which extract_version
    // unpacks.
    build_demo_app(&app, "1.1.0")?;
    let staging = dist.join("staging");
    create_dir(&staging)?;
    copy_file(&release.join(&app_exe), &staging.join(&app_exe))?;
    copy_file(&release.join(&launcher_exe), &staging.join(&launcher_exe))?;
    create_dir(&dist.join("app"))?;
    let asset = dist.join("app/demo-app-1.1.0.zip");
    write_zip(&staging, "", &asset)?;
    std::fs::remove_dir_all(&staging)
        .map_err(|err| format!("could not remove {}: {err}", staging.display()))?;

    // The installed tree: the launcher at the root under the name users
    // type, and the app inside a version folder -- the layout the launcher
    // walks.
    build_demo_app(&app, "1.0.0")?;
    let installed = dist.join("app/1.0.0");
    create_dir(&installed)?;
    copy_file(&release.join(&app_exe), &installed.join(&app_exe))?;
    copy_file(&release.join(&launcher_exe), &installed.join(&launcher_exe))?;
    copy_file(
        &release.join(&launcher_exe),
        &dist.join("app").join(&entry_exe),
    )?;

    // `sha256` is optional in the manifest format and supplied here, because
    // a download that does not match is refused outright.
    let manifest = serde_json::json!({
        "version": "1.1.0",
        "url": format!("file://{}", asset.display().to_string().replace('\\', "/")),
        "sha256": sha256_hex(&asset)?,
    });
    write_file(
        &dist.join("app/manifest.json"),
        &serde_json::to_string_pretty(&manifest).expect("a JSON value serialises"),
    )?;

    let entry = dist.join("app").join(&entry_exe);
    println!();
    println!("Installed 1.0.0 at: {}", dist.join("app").display());
    println!();
    println!("Point the app at this install and run it through the launcher:");
    println!("  DEMO_APP_INSTALL_DIR={}", dist.join("app").display());
    println!("  {}", entry.display());
    println!();
    println!("The first run reports 1.0.0 and stages 1.1.0; the second starts 1.1.0.");
    println!("  {} --version    what is installed", entry.display());
    println!(
        "  {} --previous --rollback   go back to the previous version",
        entry.display()
    );
    Ok(())
}

/// Set as real environment variables rather than left to
/// app/.cargo/config.toml alone: cargo merges [env] from every ancestor
/// directory, so a clone inside another cargo project would inherit that
/// project's values; a real environment variable outranks all of them.
fn build_demo_app(app: &Path, version: &str) -> Result {
    run_step(
        &format!("Building demo-app {version}"),
        cargo(app, &["build", "--release"])
            .env("DEMO_APP_VERSION", version)
            .env("UPGRADER_INSTALL_DIR_NAME", ".demo-app")
            .env("UPGRADER_INSTALL_ENV_VAR", "DEMO_APP_INSTALL_DIR")
            .env("UPGRADER_LAUNCHER_STEM", "demo")
            .env("UPGRADER_APP_STEM", "demo-app"),
    )
}

fn build_single_instance_app(root: &Path) -> Result {
    let app = root.join("examples/embedding/single-instance-app/app");
    run_step(
        "Building the single-instance demo",
        &mut cargo(&app, &["build", "--release"]),
    )?;
    let exe = app
        .join("target/release")
        .join(exe_name("single-instance-app"));
    println!();
    println!("Start it, then start it again from another shell:");
    println!("  {}", exe.display());
    println!("  {} --x y", exe.display());
    Ok(())
}
