//! The engine executable (structure.md): default composition, built solely
//! on runner's public builder API (ADR-011) — no access an embedder lacks.
//! Reads `bones.toml` next to wherever the executable itself lives by
//! default, or from `BONES_CONFIG` when set (paths.rs) — never the
//! process's current working directory; every field defaults to what was
//! previously hardcoded.

mod config;
mod paths;

use config::Config;

fn main() {
    if let Err(err) = run() {
        eprintln!("fatal: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::load(paths::config_path())?;

    let mut engine = runner::Engine::new()
        .extensions_dir(paths::config_relative(&config.extensions_dir))
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
    if config.audio {
        engine = engine.module(audio::Audio::new());
    }
    if config.game_core {
        engine = engine.module(game_core::GameCore::new());
    }
    if config.persistence_read_only {
        engine = engine.read_only_persistence();
    }

    engine.run().map_err(|err| err.to_string())
}
