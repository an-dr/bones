//! The reference launcher binary: the permanent entry point installed at the
//! install root, and the name users type.
//!
//! Everything it does lives in `bones_upgrader::launcher`, so a host that
//! wants its own binary name declares a `[[bin]]` like this one instead of
//! reimplementing the mechanism -- which is the expected case, since the
//! entry point carries the application's name, not the engine's.

// Same reasoning as the app: a release build is a desktop entry point, not
// a console tool, so Windows must not open a terminal behind it. Debug
// builds keep the console, which is where this file's eprintln! output goes.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;

use bones_upgrader::launcher::report_error;

fn main() {
    let Some(install_dir) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    else {
        report_error("could not resolve the launcher's own directory\n");
        std::process::exit(1);
    };
    // Everything after this executable's own name belongs to the app, not
    // to the launcher: this process is the name users type, so
    // `myapp C:/somewhere` has to reach the app unread. The launcher
    // deliberately interprets nothing, so a future app argument needs no
    // change here.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) =
        bones_upgrader::launcher::run(&bones_upgrader::host_identity(), &install_dir, &args)
    {
        report_error(&format!("{error}\n"));
        std::process::exit(1);
    }
}
