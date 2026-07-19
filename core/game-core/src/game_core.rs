//! The `GameCore` native module (design/modules.md, ADR-011, ADR-019): a
//! 2D entity/component simulation — ECS, collision, tilemap loading,
//! sprite-animation timing — composed from bought, engine-agnostic crates
//! (`hecs`, `rapier2d`, `glam`, `tiled`; see ADR-019's crate-sourcing
//! rationale). Renders by turning simulated state into `gfx/*` draw-command
//! batches, same as any other module — no rendering authority of its own.

use bus::{Envelope, Handler, Module, ModuleContext};

pub struct GameCore {
    world: hecs::World,
}

impl GameCore {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
        }
    }
}

impl Default for GameCore {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler for GameCore {
    fn handle(&mut self, _envelope: &Envelope) {
        // TODO: `game-core/*` command dispatch arrives in a later increment
        // (spawn-entity, load-tilemap) — this module has nothing to react
        // to yet.
    }
}

impl Module for GameCore {
    fn name(&self) -> &str {
        "game-core"
    }

    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe("game-core/*");
        Ok(())
    }
}

#[cfg(test)]
mod tests;
