//! Prerequisite checks. Each one either passes or names what is missing,
//! why the build needs it, and where to get it. Nothing is installed: a
//! toolchain is the developer's choice, and a script that provisions one
//! silently hides what the build depends on.

use std::fmt;
use std::process::Command;

use crate::native::{self, C_COMPILERS};
use crate::process::{cargo, is_on_path, read_output, repo_root};
use crate::Result;

/// A prerequisite the build cannot do without.
#[derive(Clone, Copy)]
pub enum Check {
    /// `rustup`, which the target and component checks query.
    Rustup,
    /// A Rust compilation target, such as `wasm32-wasip2`.
    RustTarget(&'static str),
    /// A cargo subcommand shipped as a rustup component (`fmt`, `clippy`).
    CargoComponent {
        subcommand: &'static str,
        component: &'static str,
    },
    /// A C compiler cmake can use to build SDL3.
    CCompiler,
    /// CMake itself, which drives SDL3's build.
    CMake,
    /// The Debian/Ubuntu development packages a Linux build links against.
    /// Passes elsewhere, and on distributions without `dpkg-query`, where
    /// the names in [`LINUX_PACKAGES`] are only a guide.
    LinuxPackages,
}

/// What a Linux build links against, as Debian/Ubuntu package names: SDL3's
/// video, input, and audio backends, kira's ALSA output, the GTK and
/// webkit2gtk behind wry web panels and file dialogs, and OpenSSL for the
/// upgrade module's downloads. The single list behind `doctor`, CI, and
/// `cargo xtask linux-packages`.
pub const LINUX_PACKAGES: [&str; 26] = [
    "pkg-config",
    "libasound2-dev",
    "libpulse-dev",
    "libx11-dev",
    "libxext-dev",
    "libxrandr-dev",
    "libxcursor-dev",
    "libxfixes-dev",
    "libxi-dev",
    "libxss-dev",
    "libxtst-dev",
    "libxkbcommon-dev",
    "libwayland-dev",
    "wayland-protocols",
    "libdecor-0-dev",
    "libegl1-mesa-dev",
    "libgl1-mesa-dev",
    "libgles2-mesa-dev",
    "libdrm-dev",
    "libgbm-dev",
    "libdbus-1-dev",
    "libudev-dev",
    "libibus-1.0-dev",
    "libgtk-3-dev",
    "libwebkit2gtk-4.1-dev",
    "libssl-dev",
];

// Private to this file: the report a failed Check renders, never named
// outside it, so it shares Check's file (docs/code-style.md's exception).
/// What a failed check tells the developer.
struct Missing {
    what: String,
    why: &'static str,
    install: String,
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} is missing\n  needed for: {}\n  install:    {}",
            self.what, self.why, self.install
        )
    }
}

impl Check {
    /// Every check, in the order `doctor` reports them.
    pub const ALL: [Check; 7] = [
        Check::Rustup,
        Check::RustTarget("wasm32-wasip2"),
        Check::CargoComponent {
            subcommand: "fmt",
            component: "rustfmt",
        },
        Check::CargoComponent {
            subcommand: "clippy",
            component: "clippy",
        },
        Check::CCompiler,
        Check::CMake,
        Check::LinuxPackages,
    ];

    fn label(self) -> String {
        match self {
            Check::Rustup => "rustup".to_string(),
            Check::RustTarget(target) => format!("Rust target {target}"),
            Check::CargoComponent { subcommand, .. } => format!("cargo {subcommand}"),
            Check::CCompiler => "C compiler".to_string(),
            Check::CMake => "CMake".to_string(),
            Check::LinuxPackages => "Linux development packages".to_string(),
        }
    }

    /// Runs the check; `Err` describes what to install.
    fn verify(self) -> std::result::Result<(), Missing> {
        let present = match self {
            Check::Rustup => is_on_path("rustup"),
            Check::RustTarget(target) => {
                read_output(Command::new("rustup").args(["target", "list", "--installed"]))
                    .is_some_and(|installed| installed.lines().any(|line| line.trim() == target))
            }
            Check::CargoComponent { subcommand, .. } => {
                read_output(&mut cargo(&repo_root(), &[subcommand, "--version"])).is_some()
            }
            Check::CCompiler => C_COMPILERS.iter().any(|compiler| is_on_path(compiler)),
            Check::CMake => is_on_path("cmake"),
            Check::LinuxPackages => missing_linux_packages().is_empty(),
        };
        if present {
            return Ok(());
        }
        Err(match self {
            Check::Rustup => Missing {
                what: self.label(),
                why: "adding Rust targets and components",
                install: "https://rustup.rs".to_string(),
            },
            Check::RustTarget(target) => Missing {
                what: self.label(),
                why: "compiling for that target",
                install: format!("rustup target add {target}"),
            },
            Check::CargoComponent { component, .. } => Missing {
                what: self.label(),
                why: "the formatting and lint gates",
                install: format!("rustup component add {component}"),
            },
            Check::CCompiler => Missing {
                what: self.label(),
                why: "building SDL3 from source (bones-kernel-infra)",
                install: if cfg!(windows) {
                    "Visual Studio Build Tools with \"Desktop development with C++\" \
                     (https://visualstudio.microsoft.com/downloads/), or LLVM clang \
                     (winget install LLVM.LLVM)"
                        .to_string()
                } else {
                    "gcc or clang from your system package manager".to_string()
                },
            },
            Check::CMake => Missing {
                what: self.label(),
                why: "building SDL3 from source (bones-kernel-infra)",
                install: if cfg!(windows) {
                    "https://cmake.org/download/ or winget install Kitware.CMake".to_string()
                } else {
                    "cmake from your system package manager, or https://cmake.org/download/"
                        .to_string()
                },
            },
            Check::LinuxPackages => Missing {
                what: format!("{} ({})", self.label(), missing_linux_packages().join(" ")),
                why: "linking SDL3, audio, and the web panels on Linux",
                install: "sudo apt-get install $(cargo xtask linux-packages)".to_string(),
            },
        })
    }
}

/// Prepares the native build environment, then stops at the first failed
/// check. Commands call this before doing any work.
pub fn require(checks: &[Check]) -> Result {
    native::configure();
    for check in checks {
        check.verify().map_err(|missing| missing.to_string())?;
    }
    Ok(())
}

/// `cargo xtask doctor`: runs every check and reports all of them, so a new
/// machine is set up in one pass rather than one failure at a time.
pub fn run_doctor() -> Result {
    native::configure();
    let mut missing = 0;
    for check in Check::ALL {
        match check.verify() {
            Ok(()) => println!("ok       {}", check.label()),
            Err(report) => {
                missing += 1;
                println!("MISSING  {report}");
            }
        }
    }
    match missing {
        0 => {
            println!("\nEverything the build needs is installed.");
            Ok(())
        }
        n => Err(format!("{n} prerequisite(s) missing; see above")),
    }
}

/// The entries of [`LINUX_PACKAGES`] `dpkg-query` reports as not installed;
/// empty off Linux or where `dpkg-query` is absent.
fn missing_linux_packages() -> Vec<&'static str> {
    if !cfg!(target_os = "linux") || !is_on_path("dpkg-query") {
        return Vec::new();
    }
    LINUX_PACKAGES
        .into_iter()
        .filter(|package| {
            read_output(Command::new("dpkg-query").args(["-W", "-f=${Status}", package]))
                .is_none_or(|status| !status.contains("install ok installed"))
        })
        .collect()
}
