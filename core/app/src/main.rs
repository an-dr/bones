//! The engine executable (structure.md): default composition, built solely
//! on runner's public builder API (ADR-011) — no access an embedder lacks.

fn main() {
    if let Err(err) = runner::Engine::builder()
        .extensions_dir("extensions")
        .window("bones", 800, 600)
        .run()
    {
        eprintln!("fatal: {err}");
        std::process::exit(1);
    }
}
