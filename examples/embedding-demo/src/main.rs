//! Proves the builder API needs no privileged access (design/modules.md,
//! ADR-017): a separate crate depending on bones's crates the same way any
//! embedder would, injecting its own native module and building its own
//! binary — none of this reaches past `bones-engine`, only `Engine`,
//! `Module`, `Handler`, and `ModuleContext`. Run with: `cargo run` from
//! this directory.

use std::time::{Duration, Instant};

use bones_engine::bus::{Envelope, Handler, Module, ModuleContext};
use bones_engine::logging::Logger;

/// Logs the wall-clock time once a second, driven by `core/tick` — proves
/// a custom native module gets `init`, its requested subscription, and bus
/// deliveries the same way `renderer` does.
struct Clock {
    logger: Logger,
    last_logged: Instant,
}

impl Clock {
    fn new(logger: Logger) -> Self {
        Self {
            logger,
            last_logged: Instant::now() - Duration::from_secs(1),
        }
    }
}

impl Handler for Clock {
    fn handle(&mut self, _envelope: &Envelope) {
        if self.last_logged.elapsed() >= Duration::from_secs(1) {
            self.last_logged = Instant::now();
            self.logger.info("clock", "tick");
        }
    }
}

impl Module for Clock {
    fn name(&self) -> &str {
        "clock"
    }

    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe("core/tick");
        Ok(())
    }
}

fn main() -> bones_engine::Result<()> {
    let logger = Logger::default();
    bones_engine::Engine::new()
        .logger(logger.clone())
        .module(Clock::new(logger))
        .run()
}
