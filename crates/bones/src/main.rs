//! The engine executable (structure.md): default composition, built solely
//! on runner's public builder API (ADR-011) — no access an embedder lacks.
//! Reads `bones.toml` next to wherever the executable itself lives by
//! default, or from `BONES_CONFIG` when set (paths.rs) — never the
//! process's current working directory; every field defaults to what was
//! previously hardcoded.

// A release build is a windowed application:
// - without this, Windows opens a console behind the SDL window;
// - debug builds keep the console subsystem, which is where their log output
//   goes while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
#[cfg(windows)]
mod parent_console;
mod paths;

use config::Config;

fn main() {
    #[cfg(windows)]
    parent_console::attach();
    if let Err(err) = run() {
        eprintln!("fatal: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::load(paths::config_path())?;

    let mut engine = bones_kernel::Kernel::new()
        .logger(logger())
        .extensions_dir(paths::config_relative(&config.extensions_dir))
        .extension_budget(bones_kernel::bus::BudgetLimits {
            max_inbound: config.extension_max_inbound,
            max_publishes: config.extension_max_publishes,
        })
        .extension_load_timeout(std::time::Duration::from_millis(
            config.extension_load_timeout_ms,
        ))
        .extension_call_timeout(std::time::Duration::from_millis(
            config.extension_call_timeout_ms,
        ))
        .window(
            config.window_title,
            config.window_width,
            config.window_height,
        )
        .saves_dir(paths::config_relative(&config.saves_dir));
    if config.renderer {
        engine = engine.renderer();
    }
    if config.ui {
        engine = engine.ui();
    }
    if config.web {
        #[cfg(feature = "web")]
        {
            engine = engine.web();
        }
        #[cfg(not(feature = "web"))]
        {
            return Err("web = true requires building app with --features web".to_string());
        }
    }
    if config.audio {
        engine = engine.module(bones_kernel::audio::Audio::new());
    }
    if config.game_core {
        engine = engine.module(bones_kernel::game_core::GameCore::new());
    }
    // For screenshots where there is no display, or none should be used.
    engine = engine.offscreen_from_env(OFFSCREEN_ENV_VAR);
    // Off unless the variable names a port. When it does, devtools also
    // becomes the extension controller, so its routes can reload extensions.
    engine = engine.devtools_from_env(DEVTOOLS_ENV_VAR);
    if bones_kernel::devtools::Devtools::requested(DEVTOOLS_ENV_VAR) {
        engine = engine.extension_controller("devtools");
    }
    for (name, patterns) in config.deny {
        engine =
            engine.extension_permissions(name, bones_kernel::wasm::Permissions::deny(patterns));
    }
    if config.persistence_read_only {
        engine = engine.read_only_persistence();
    }

    engine.run().map_err(|err| err.to_string())
}

/// JSON lines when `BONES_LOG_FORMAT=json`, for a tool to parse; plain text
/// otherwise. Lines below `BONES_LOG_LEVEL` (default `info`) are dropped,
/// which keeps the runner's per-tick debug line out of the way.
fn logger() -> bones_kernel::logging::Logger {
    use bones_kernel::logging::{JsonSink, Level, LevelFilter, LogSink, Logger, StdoutSink};
    use std::sync::Arc;
    let sink: Arc<dyn LogSink> = match std::env::var(LOG_FORMAT_ENV_VAR).as_deref() {
        Ok("json") => Arc::new(JsonSink),
        _ => Arc::new(StdoutSink),
    };
    let min = std::env::var(LOG_LEVEL_ENV_VAR)
        .ok()
        .and_then(|name| LevelFilter::parse(&name))
        .unwrap_or(Level::Info);
    Logger::new(Arc::new(LevelFilter::new(sink, min)))
}

/// Selects the quietest level written; see the crate README.
const LOG_LEVEL_ENV_VAR: &str = "BONES_LOG_LEVEL";

/// Names the devtools channel's port; see the crate README.
const DEVTOOLS_ENV_VAR: &str = "BONES_DEVTOOLS_PORT";

/// Opens the window in memory instead of on a display; see the crate README.
const OFFSCREEN_ENV_VAR: &str = "BONES_OFFSCREEN";

/// Selects the log format; see the crate README.
const LOG_FORMAT_ENV_VAR: &str = "BONES_LOG_FORMAT";
