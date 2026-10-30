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
//! builder methods on [`Kernel`] and are deliberately absent — an embedder
//! selects them with `.renderer()`, `.ui()`, or `.web()` and never names their
//! types.
//!
//! What an embedder *must* be able to name is here instead, because the
//! signatures of this crate's own API demand it: [`Error`] and [`Result`] for
//! what [`Kernel::build`] and [`Kernel::run`] return, [`bus::PlatformEvent`]
//! for the argument of the hook a custom module overrides, and
//! [`platform::Platform`] for the value [`BuiltKernel`] hands back. A facade
//! that returns types the caller cannot write down is not a boundary, so none
//! of these require adding `sdl3`, `wasmtime`, or `bones-kernel-infra` to a
//! consumer's manifest.
//!
//! # Writing a native module
//!
//! A native module is a plain type implementing two traits from [`bus`]. On the
//! bus it is indistinguishable from a WASM extension (ADR-011) — same topics,
//! same delivery rules — it simply runs natively and in-process.
//!
//! ```
//! use bones_kernel::bus::{Envelope, Handler, Module, ModuleContext};
//! use bones_kernel::logging::Logger;
//! use bones_kernel::Kernel;
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
//! # fn compose() -> bones_kernel::Result<()> {
//! let logger = Logger::default();
//! let engine = Kernel::new()
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
//! type above is named through `bones_kernel`, so if one stops being reachable
//! here the documentation stops building rather than going quietly wrong.

// A public item with no documentation is an unfinished promise: this crate is
// what the engine commits to keeping working, so `cargo xtask test`'s clippy gate
// (`-D warnings`) turns any gap here into a build failure.
#![warn(missing_docs)]

mod kernel;

/// Composition: build an engine, then run or step it.
pub use kernel::{BuiltKernel, Kernel};

/// What [`Kernel::build`] and [`Kernel::run`] fail with.
///
/// The engine's fallible surface is uniform: every failure — a missing
/// `.window(...)`, an invalid tick rate, a component that will not
/// instantiate — arrives as one of these, carrying a chain of causes rather
/// than an enum of variants the facade would have to keep in step with every
/// layer beneath it.
pub use anyhow::Error;

/// The result type the engine's fallible calls return, so an embedder can
/// write `fn start() -> bones_kernel::Result<()>` without naming [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Lifecycle, from the kernel infrastructure: the frame loop is module-agnostic,
/// so it lives in `bones-kernel-infra` and is surfaced here rather than
/// defined here.
pub use bones_kernel_infra::runner::{read_tick_dt, Runner};

/// The extension host (ADR-040), present with the `wasm` feature.
///
/// The builder's extension methods — `extensions_dir`, `startup_extension`,
/// `extension_controller`, and the budgets — register it for you. Name it
/// directly only to compose it by hand with `.module(...)`.
#[cfg(feature = "wasm")]
pub mod wasm {
    pub use bones_module_wasm::host::ExtensionTimeouts;
    pub use bones_module_wasm::{Permissions, Wasm};
}

/// The message bus, and the traits a native module implements to join it.
pub mod bus {
    pub use bones_kernel_infra::bus::{
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
    pub use bones_kernel_infra::bus::{offer_event, PlatformEvent};
}

/// The OS window and its event source (design/platform.md).
///
/// Present because [`BuiltKernel::platform`](crate::BuiltKernel) hands one
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
/// use bones_kernel::bus::{Envelope, Handler, Module, ModuleContext, PlatformEvent};
/// use bones_kernel::platform::{Platform, WindowSurface};
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
    pub use bones_kernel_infra::platform::{Platform, WindowSurface};
}

/// The `window-control` service (ADR-035): bring the window to the front
/// without owning it. A custom module reads it with `get_service`.
pub mod window_control {
    pub use bones_kernel_infra::window_control::WindowControl;
}

/// The `exit-request` service (ADR-040): ask the kernel to begin orderly
/// shutdown from any module. [`BuiltKernel::exit_requested`](crate::BuiltKernel)
/// is the same flag.
pub mod exit {
    pub use bones_kernel_infra::exit::ExitRequest;
}

/// The `frame-control` service (design/modules.md): pause the simulation and
/// step it frame by frame. [`BuiltKernel::frame_control`](crate::BuiltKernel)
/// is the same control.
pub mod frame_control {
    pub use bones_kernel_infra::frame_control::FrameControl;
}

/// The `display-info` service (ADR-040): the display's fullscreen modes and
/// desktop resolution, queried once before any window hand-off.
pub mod display {
    pub use bones_kernel_infra::display::DisplayInfo;
}

/// Single instance per user (ADR-035): claim the application's `app_uid` in `main`, before the
/// engine opens a window, then pass the [`instance::Primary`] to
/// [`Kernel::single_instance`].
///
/// ```no_run
/// use std::time::Duration;
/// use bones_kernel::instance::{claim, Claim, Handoff};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let handoff = Handoff {
///     args: std::env::args().skip(1).collect(),
///     cwd: std::env::current_dir()?,
/// };
/// match claim("myapp", handoff, Duration::from_secs(2))? {
///     Claim::HandedOff => return Ok(()),
///     Claim::Primary(primary) => bones_kernel::Kernel::new().single_instance(primary).run()?,
/// }
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "instance")]
pub mod instance {
    pub use bones_module_single_instance::{claim, Claim, Handoff, Primary};
}

/// Desktop registration (ADR-036): what [`Kernel::desktop`] registers, and
/// where, plus the functions an application's own `--install` calls
/// directly, before or without a kernel.
#[cfg(feature = "desktop")]
pub mod desktop {
    pub use bones_module_desktop::{is_registered, register, unregister, Integration, Locations};
}

/// Application-side upgrading (ADR-039): what [`Kernel::upgrade`] updates
/// from.
#[cfg(feature = "upgrade")]
pub mod upgrade {
    pub use bones_module_upgrade::{
        AppIdentity, CompanionOutcome, CompanionPolicy, Fetch, HttpsFetch, ReleaseSource,
        UpgradeConfig, UpgradeModule,
    };
}

/// Starting programs and terminals from an application's native code: a
/// plain library with no bus module, because a guest that could start any
/// program would reach past the sandbox.
#[cfg(feature = "launch")]
pub mod launch {
    pub use bones_kernel_launch::{launch, open_terminal, resolve, Launched, Program};
}

/// Watching folders from an application's native code, with bursts of
/// changes debounced into one report per watch. A plain library with no bus
/// module.
#[cfg(feature = "watch")]
pub mod watch {
    pub use bones_kernel_watch::{Change, ChangeKind, WatchId, Watcher, DEFAULT_QUIET};
}

/// The loopback devtools channel and its log history, for a composition
/// that builds the module itself; [`Kernel::devtools_from_env`] with
/// [`Kernel::devtools_redacting`] covers the rest.
#[cfg(feature = "devtools")]
pub mod devtools {
    pub use bones_module_devtools::{Devtools, LogHistory};
}

/// The `draw-target` service (ADR-031): what a module owning a drawing
/// surface offers to a module that has pixels but no surface.
///
/// Re-exported because it is a contract between two *replaceable* modules —
/// an embedder substituting either side implements or consumes these, and
/// could not name them otherwise.
#[cfg(feature = "presentation")]
pub mod draw_target {
    pub use bones_kernel_infra::draw_target::{DrawTarget, DrawTargetService, UiMesh, UiVertex};
}

/// Structured logging, including the sink trait the kernel infrastructure resolves against
/// (ADR-012).
pub mod logging {
    pub use bones_kernel_infra::logging::{
        JsonSink, Level, LevelFilter, LogSink, Logger, RecordingSink, StdoutSink,
    };
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
