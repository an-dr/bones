//! `cargo xtask dist [--no-archive]`: builds the engine, assembles a
//! runnable dist/ and a versioned, checksummed release archive beside it.
//!
//! - examples/ is deliberately not built here: a distribution is the engine,
//!   the ABI it implements, and an empty extensions/ folder, not a demo
//!   bundle. `cargo xtask example <name>` builds one example.
//! - `--no-archive` stops after dist/, for the inner-loop case of running
//!   the engine you just built.

use std::fmt::Write as _;
use std::path::Path;

use crate::archive::{sha256_hex, write_zip};
use crate::files::{copy_dir, copy_file, create_dir, exe_name, recreate_dir, write_file};
use crate::metadata::{abi_version, cargo_metadata, host_triple, package_version, packages};
use crate::prereq::{self, Check};
use crate::process::{cargo, repo_root, run_step};
use crate::Result;

/// A sample configuration with every field at the value the engine would
/// use anyway, so the file documents the defaults instead of changing
/// behaviour by existing. Keep it in step with crates/bones/src/config.rs.
const SAMPLE_CONFIG: &str = include_str!("../templates/bones.toml");
const RELEASE_README: &str = include_str!("../templates/README.md");

pub fn run(args: &[String]) -> Result {
    let archive = match args {
        [] => true,
        [flag] if flag == "--no-archive" => false,
        _ => return Err("usage: cargo xtask dist [--no-archive]".to_string()),
    };
    prereq::require(&[Check::CCompiler, Check::CMake])?;

    let root = repo_root();
    let version = package_version(&root, "bones")?;
    let abi = abi_version(&root)?;
    // A bundle carries a native executable and a statically linked SDL, so
    // it is only valid for the OS and architecture it was built on -- naming
    // it accordingly is what stops the wrong one being shipped.
    let (os, arch) = (platform_os(), platform_arch());
    let bundle_name = format!("bones-{version}-{os}-{arch}");
    let exe = exe_name("bones");
    let dist = root.join("dist");

    run_step(
        "Building app",
        &mut cargo(&root, &["build", "-p", "bones", "--release"]),
    )?;

    recreate_dir(&dist)?;
    // extensions/ ships empty: it is where a user drops their own .wasm files.
    create_dir(&dist.join("extensions"))?;
    copy_file(&root.join("target/release").join(&exe), &dist.join(&exe))?;

    // The ABI ships with the engine that implements it, so the
    // bones:extension version in it is verifiable rather than advisory --
    // wasmtime refuses a component whose imported interface version differs.
    // The whole directory: an author outside Rust also needs wire-format.md
    // and vectors/ to build and check payloads.
    println!("==> Copying the extension ABI...");
    copy_dir(&root.join("wit"), &dist.join("wit"))?;

    // The MIT licence requires its notice to travel with every copy, so a
    // bundle without it is not merely untidy -- it is not licensed.
    println!("==> Copying licence and notices...");
    copy_file(&root.join("LICENSE"), &dist.join("LICENSE"))?;
    write_file(
        &dist.join("THIRD-PARTY-NOTICES.txt"),
        &third_party_notices(&root, &version, os, arch)?,
    )?;

    println!("==> Writing the sample configuration and release README...");
    write_file(&dist.join("bones.toml"), SAMPLE_CONFIG)?;
    write_file(
        &dist.join("README.md"),
        &RELEASE_README
            .replace("{version}", &version)
            .replace("{os}", os)
            .replace("{arch}", arch)
            .replace("{abi}", &abi)
            .replace("{exe}", &exe),
    )?;

    println!();
    println!(
        "Distribution ready: {} (extensions in {})",
        dist.join(&exe).display(),
        dist.join("extensions").display()
    );
    println!("Extension ABI: dist/wit/extension.wit (bones:extension@{abi})");
    if archive {
        write_release_archive(&root, &dist, &bundle_name)?;
    }
    Ok(())
}

/// Zips dist/ under a directory named after the bundle -- so unpacking never
/// scatters files into the current directory -- and writes its checksum.
fn write_release_archive(root: &Path, dist: &Path, bundle_name: &str) -> Result {
    println!("==> Archiving {bundle_name}...");
    let archive_name = format!("{bundle_name}.zip");
    let archive = root.join(&archive_name);
    write_zip(dist, bundle_name, &archive)?;

    // This verifies a *download*, not a *build*: zip entries carry
    // modification times, so two runs produce different hashes from
    // identical inputs. Byte-reproducible builds are separate work
    // (docs/roadmap.md).
    let hash = sha256_hex(&archive)?;
    write_file(
        &root.join(format!("{archive_name}.sha256")),
        &format!("{hash}  {archive_name}\n"),
    )?;
    println!("Release archive: {archive_name}");
    println!("SHA256: {hash}");
    Ok(())
}

/// Generated rather than maintained by hand: the engine links its whole
/// dependency tree statically, so the list changes whenever a dependency
/// does. cargo metadata is the source because it needs no extra tool.
fn third_party_notices(root: &Path, version: &str, os: &str, arch: &str) -> Result<String> {
    let metadata = cargo_metadata(root, &["--filter-platform", &host_triple()?])?;
    let mut dependencies: Vec<_> = packages(&metadata)
        .filter(|package| !package["source"].is_null())
        .collect();
    dependencies.sort_by_key(|package| {
        (
            package["name"].as_str().unwrap_or_default().to_string(),
            package["version"].as_str().unwrap_or_default().to_string(),
        )
    });

    let mut notices = format!(
        "Third-party notices for bones {version} ({os}-{arch})\n\
         \n\
         The bones executable statically links the packages below. Each remains under\n\
         its own licence; this file lists them so a redistributor can honour those\n\
         terms. bones itself is MIT -- see LICENSE.\n\
         \n\
         Generated by cargo xtask dist from cargo metadata, for the host triple this\n\
         bundle was built for. Licences are identified by SPDX expression rather than\n\
         reproduced; where a licence requires its full text to travel with a binary,\n\
         obtain that text from the package's own repository, linked below.\n\
         \n"
    );
    for package in dependencies {
        let license = match package["license"].as_str() {
            Some(license) => license.to_string(),
            None => format!("see {}", package["license_file"].as_str().unwrap_or("")),
        };
        let _ = writeln!(
            notices,
            "{} {}",
            str_of(package, "name"),
            str_of(package, "version")
        );
        let _ = writeln!(notices, "  licence:    {license}");
        if let Some(repository) = package["repository"].as_str() {
            let _ = writeln!(notices, "  repository: {repository}");
        }
        notices.push('\n');
    }
    Ok(notices)
}

fn str_of<'a>(package: &'a serde_json::Value, field: &str) -> &'a str {
    package[field].as_str().unwrap_or_default()
}

fn platform_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macos",
        "windows" => "windows",
        _ => "linux",
    }
}

fn platform_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    }
}
