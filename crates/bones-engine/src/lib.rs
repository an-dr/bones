//! The public library surface of bones: the one crate an embedder depends on.
//!
//! Everything reachable from here is intended for embedders; everything not
//! reachable is an implementation detail, and the dependency graph — not a
//! convention — is what enforces that. `bones` (the engine executable) depends
//! on this crate and nothing else under `core/`, so structure.md's rule that
//! the app has no access an embedder lacks holds by construction.
//!
//! The surface is curated rather than a glob of every internal crate. Adding
//! to it is a deliberate act, which is the point: what is re-exported here is
//! what the engine promises to keep working.
//!
//! Presentation backends (`renderer`, `ui`, `web`, `platform`) are composed
//! through builder methods on [`Engine`] and are deliberately absent — an
//! embedder selects them with `.renderer()`, `.ui()`, or `.web()` and never
//! names their types.

mod engine;

/// Composition: build an engine, then run or step it.
pub use engine::{BuiltEngine, Engine};

/// Lifecycle, from the kernel: the frame loop and the extension supervisor
/// are module-agnostic, so they live in `bones-kernel` and are surfaced
/// here rather than defined here.
pub use bones_kernel::runner::{read_tick_dt, Runner};
pub use bones_kernel::wasm_extensions::supervisor::Supervisor;

/// The message bus, and the traits a native module implements to join it.
pub mod bus {
    pub use bones_kernel::bus::{
        BudgetLimits, Bus, DropCounters, Endpoint, EndpointBudget, Envelope, Handler, Module,
        ModuleContext, ModuleRegistration, Registry, Respond, SendError, ServiceRegistry,
    };
}

/// Structured logging, including the sink trait the kernel resolves against
/// (ADR-012).
pub mod logging {
    pub use bones_kernel::logging::{Level, LogSink, Logger, RecordingSink, StdoutSink};
}

/// The typed core messages and their payload codecs.
///
/// This is the ABI version line, not the engine's: `bones-messages` moves only
/// when the guest contract changes, so its version is deliberately not the one
/// this crate carries. It is re-exported because a native module speaks the
/// same vocabulary a WASM guest does.
pub mod messages {
    pub use bones_messages::*;
}

/// Sound effects and music, backed by kira.
#[cfg(feature = "audio")]
pub mod audio {
    pub use bones_module_audio::Audio;
}

/// The 2D simulation module: ECS, collision, tilemaps, sprite animation
/// (ADR-019, ADR-022).
#[cfg(feature = "game-core")]
pub mod game_core {
    pub use bones_module_game_core::{
        load_collision_rects, BodyHandle, BodyKind, Collider, ColliderHandle, CollisionRect,
        GameCore, PhysicsBackend, PhysicsWorldKind, Rapier2dBackend, RetroBackend, SpriteAnimation,
        SpriteTint, SquareColor, Transform, WorldBody,
    };
}
