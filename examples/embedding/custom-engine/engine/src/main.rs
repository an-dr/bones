//! `custom-engine`: the bones executable, plus one native capability.
//!
//! The point of this example is what the composition below *is not* — it is not
//! a different program. It is the same stack `crates/bones/src/main.rs` builds
//! (a window, the renderer, the egui ui, a directory of WASM extensions) with
//! one extra `.module(...)` line. Put the two files side by side: that line,
//! and the vocabulary crate it speaks, are the entire difference between the
//! shipped engine and your own.
//!
//! That is the answer to "why embed". Extensions cover product behaviour;
//! embedding covers the case where the capability itself does not exist yet,
//! because it needs native access the sandbox refuses to grant.
//!
//! Run it with `pwsh build.ps1`, which packages this binary together with both
//! the custom extension and `hello`, so a native module and two WASM guests are
//! visibly running in one process.

mod host_facts;

use bones_engine::logging::Logger;

use host_facts::HostFacts;

fn main() {
    if let Err(error) = run() {
        eprintln!("fatal: {error}");
        std::process::exit(1);
    }
}

fn run() -> bones_engine::Result<()> {
    let logger = Logger::default();

    bones_engine::Engine::new()
        .logger(logger.clone())
        // Everything from here to `.ui()` is what the shipped `bones` binary
        // composes. `extensions_dir` is relative to the executable, not the
        // shell's working directory, so the packaged dist/ runs the same way
        // however it was launched.
        .extensions_dir("extensions")
        .window("bones with a custom module", 900, 600)
        .renderer()
        .ui()
        // ...and this is the whole difference.
        .module(HostFacts::new(logger))
        .run()
}
