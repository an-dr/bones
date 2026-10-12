//! The launcher half: the permanent entry point users actually run.
//!
//! It exists because an application cannot replace its own running
//! executable. This binary never changes, lives at the install root under the
//! name users type, and its only job is to find the newest version folder and
//! start the app inside it.
//!
//! `bones_launcher::run` selects a version and forwards application arguments.
//! `--previous` selects the previous version without modifying the installation.
//! A host declares the binary so it carries the application's name rather than
//! the engine's.

use std::path::Path;

use bones_launcher::report_error;

fn main() {
    let Some(install_dir) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    else {
        report_error("could not resolve the launcher's own directory\n");
        std::process::exit(1);
    };
    let stem = option_env!("UPGRADER_APP_STEM").unwrap_or("demo-app");
    let app_exe = if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = bones_launcher::run(&app_exe, &install_dir, &args) {
        report_error(&format!("{error}\n"));
        std::process::exit(1);
    }
}
