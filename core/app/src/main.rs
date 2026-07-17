//! The engine executable (structure.md): default composition, built solely
//! on runner's public builder API (ADR-011) — no access an embedder lacks.
//! Reads `bones.toml` next to wherever it runs (config.rs); every field
//! defaults to what was previously hardcoded.

mod config;

use config::Config;

fn main() {
    if let Err(err) = run() {
        eprintln!("fatal: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::load("bones.toml")?;

    let mut engine = runner::Engine::new()
        .extensions_dir(config.extensions_dir)
        .window(config.window_title, config.window_width, config.window_height);
    if config.renderer {
        engine = engine.renderer();
    }

    engine.run().map_err(|err| err.to_string())
}
