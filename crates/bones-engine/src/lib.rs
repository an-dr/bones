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
//! Presentation *backends* (`renderer`, `ui`, `web`) are composed through
//! builder methods on [`Engine`] and are deliberately absent — an embedder
//! selects them with `.renderer()`, `.ui()`, or `.web()` and never names their
//! types.
//!
//! What an embedder *must* be able to name is here instead, because the
//! signatures of this crate's own API demand it: [`Error`] and [`Result`] for
//! what [`Engine::build`] and [`Engine::run`] return, [`bus::PlatformEvent`]
//! for the argument of the hook a custom module overrides, and
//! [`platform::Platform`] for the value [`BuiltEngine`] hands back. A facade
//! that returns types the caller cannot write down is not a boundary, so none
//! of these require adding `sdl3`, `wasmtime`, or `bones-kernel` to a
//! consumer's manifest.
//!
//! # Writing a native module
//!
//! A native module is a plain type implementing two traits from [`bus`]. On the
//! bus it is indistinguishable from a WASM extension (ADR-011) — same topics,
//! same delivery rules — it simply runs natively and in-process.
//!
//! ```
//! use bones_engine::bus::{Envelope, Handler, Module, ModuleContext};
//! use bones_engine::logging::Logger;
//! use bones_engine::Engine;
//!
//! struct Clock {
//!     logger: Logger,
//!     ticks: u64,
//! }
//!
//! // Bus deliveries arrive here, for whatever `init` subscribed to.
//! impl Handler for Clock {
//!     fn handle(&mut self, _envelope: &Envelope) {
//!         self.ticks += 1;
//!     }
//! }
//!
//! impl Module for Clock {
//!     fn name(&self) -> &str {
//!         "clock"
//!     }
//!
//!     // Runs once at build time, in registration order: request
//!     // subscriptions and resolve services here.
//!     fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
//!         ctx.subscribe("core/tick");
//!         self.logger.info("clock", "ready");
//!         Ok(())
//!     }
//! }
//!
//! # fn compose() -> bones_engine::Result<()> {
//! let logger = Logger::default();
//! let engine = Engine::new()
//!     .logger(logger.clone())
//!     .module(Clock { logger, ticks: 0 })
//!     .build()?;
//! # let _ = engine;
//! # Ok(())
//! # }
//! ```
//!
//! `render`, `present`, `respond`, `shutdown`, and `filter_event` are the other
//! hooks, each defaulting to a no-op — a module overrides only what it needs.
//! [`platform`] has an example of the input hook.
//!
//! These examples are compiled by the test suite, which is deliberate: every
//! type above is named through `bones_engine`, so if one stops being reachable
//! here the documentation stops building rather than going quietly wrong.

// A public item with no documentation is an unfinished promise: this crate is
// what the engine commits to keeping working, so `test.ps1`'s clippy gate
// (`-D warnings`) turns any gap here into a build failure.
#![warn(missing_docs)]

mod engine;

/// Composition: build an engine, then run or step it.
pub use engine::{BuiltEngine, Engine};

/// What [`Engine::build`] and [`Engine::run`] fail with.
///
/// The engine's fallible surface is uniform: every failure — a missing
/// `.window(...)`, an invalid tick rate, a component that will not
/// instantiate — arrives as one of these, carrying a chain of causes rather
/// than an enum of variants the facade would have to keep in step with every
/// layer beneath it.
pub use wasmtime::Error;

/// The result type the engine's fallible calls return, so an embedder can
/// write `fn start() -> bones_engine::Result<()>` without naming [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

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

    /// The raw platform event a module's `filter_event` hook is offered, and
    /// the function that offers it to the modules topmost first (ADR-008).
    ///
    /// `run` calls `offer_event` itself; it is public so a caller driving
    /// [`Runner::step`](crate::Runner::step) can route input on exactly the
    /// same terms instead of reimplementing the layering rule.
    #[cfg(feature = "presentation")]
    pub use bones_kernel::bus::{offer_event, PlatformEvent};
}

/// The OS window and its event source (design/platform.md).
///
/// Present because [`BuiltEngine::platform`](crate::BuiltEngine) hands one
/// back, not because an embedder is expected to build one: `.window(...)`
/// creates it, and a module that wants the window itself consumes the
/// [`WindowSurface`](platform::WindowSurface) service rather than reaching
/// through here.
///
/// # Claiming input before it reaches the scene
///
/// A module that draws something interactive overrides `filter_event` to take
/// the events landing on it. Modules are offered each event topmost-first —
/// reverse registration order, so whatever was composed last is asked first
/// (ADR-008, ADR-031) — and a claimed event never becomes an `input/*` message,
/// so the game underneath never sees it.
///
/// ```
/// use bones_engine::bus::{Envelope, Handler, Module, ModuleContext, PlatformEvent};
/// use bones_engine::platform::{Platform, WindowSurface};
///
/// struct Overlay {
///     grabbing: bool,
/// }
///
/// impl Handler for Overlay {
///     fn handle(&mut self, _envelope: &Envelope) {}
/// }
///
/// impl Module for Overlay {
///     fn name(&self) -> &str {
///         "overlay"
///     }
///
///     fn init(&mut self, _ctx: &mut ModuleContext) -> Result<(), String> {
///         Ok(())
///     }
///
///     fn filter_event(&mut self, event: &PlatformEvent) -> bool {
///         // Returning true claims the event. Return false for anything this
///         // module only wants to observe.
///         match event {
///             PlatformEvent::MouseButtonDown { .. } if self.grabbing => true,
///             _ => false,
///         }
///     }
/// }
///
/// // A replacement renderer consumes the window itself. It is named, not
/// // stored: `WindowSurface` is not `Send` while `Handler` is, so a module
/// // keeping the window across frames needs its own thread-affinity wrapper.
/// fn take_window(ctx: &mut ModuleContext) -> Option<WindowSurface> {
///     ctx.consume_service::<WindowSurface>()
/// }
///
/// // A custom driver, rather than calling `run`, needs to name this too.
/// fn is_headless(platform: &Option<Platform>) -> bool {
///     platform.is_none()
/// }
/// # let _ = (take_window as fn(&mut ModuleContext) -> Option<WindowSurface>, is_headless as fn(&Option<Platform>) -> bool);
/// ```
#[cfg(feature = "presentation")]
pub mod platform {
    pub use bones_kernel::platform::{Platform, WindowSurface};
}

/// The `draw-target` service (ADR-031): what a module owning a drawing
/// surface offers to a module that has pixels but no surface.
///
/// Re-exported because it is a contract between two *replaceable* modules —
/// an embedder substituting either side implements or consumes these, and
/// could not name them otherwise.
#[cfg(feature = "presentation")]
pub mod draw_target {
    pub use bones_kernel::draw_target::{DrawTarget, DrawTargetService, UiMesh, UiVertex};
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
