use std::sync::{Arc, Mutex};

use bones_kernel_infra::bus::{Envelope, Module};
use bones_kernel_infra::exit::ExitRequest;
use bones_kernel_infra::frame_control::FrameControl;
use bones_messages::window::CloseRequested;
use bones_messages::{EncodeMessage, Message};

use bones_kernel_infra::bus::Registry;
use bones_kernel_infra::runner::Runner;

/// The sender the kernel stamps on what it publishes itself, such as the
/// close request on an exit it decided. Guests see it as the sender.
pub(super) const ENGINE_SENDER: &str = "engine";

/// Everything `Kernel::build` wires up: the step-driven `Runner`, the
/// platform window (if `.window(...)` was set), every native module —
/// `.renderer()`'s and `.ui()`'s included, since those are sugar over
/// `.module(...)` — and the call registry they all answer on. Extensions,
/// when the `wasm` feature is on, are hosted by one of those modules
/// (ADR-040).
///
/// Modules are held as `dyn Module` rather than as typed fields. An
/// embedder drives them through the frame-phase methods below; nothing
/// here exposes a type `bones-kernel` does not also re-export.
///
/// **The fields are public, and that is the stable API.** An embedder driving
/// its own loop instead of calling `run` needs all of them at once, and
/// destructuring is how it takes them — `let BuiltKernel { runner, modules,
/// .. } = engine.build()?;`. Accessors would return borrows of one value and
/// make that impossible, so the fields stay public and are covered by the
/// engine version line like any other public item.
pub struct BuiltKernel {
    /// The frame loop (ADR-014): owns the bus, dispatches, and ticks
    /// subscribers. Drive it with `step(dt)` for one frame's worth of work.
    pub runner: Runner,
    /// The OS window and its event source, present only if `.window(...)`
    /// was set. `None` is a headless engine, which is a supported
    /// composition and not a failure — see [`BuiltKernel::is_headless`].
    #[cfg(feature = "presentation")]
    pub platform: Option<crate::platform::Platform>,
    /// Every registered native module, in registration order.
    ///
    /// That order is the layering: `render` and `present` walk it forward so
    /// a later module draws above an earlier one, and input walks it backward
    /// so the topmost is offered an event first (ADR-031).
    pub modules: Vec<Arc<Mutex<Box<dyn Module>>>>,
    /// Every endpoint that answers a direct `send` (ADR-010), by name:
    /// modules, and extensions while they are loaded. Call one with
    /// `registry.call(sender, name, payload)`.
    pub registry: Registry,
    /// Set by any loaded extension's `request-exit` host-api call, or by any
    /// module through the `exit-request` service; `run`'s loop breaks once it
    /// is set, the same as `Platform::quit_requested()`.
    pub exit_requested: ExitRequest,
    /// The `frame-control` service: whoever holds a clone can pause the
    /// simulation and step it. `run` obeys it; a caller driving `step(dt)`
    /// itself decides whether to.
    pub frame_control: FrameControl,
    pub(super) shutdown_started: bool,
}

impl BuiltKernel {
    /// True when the engine was built without a native presentation stack.
    pub fn is_headless(&self) -> bool {
        #[cfg(feature = "presentation")]
        {
            self.platform.is_none()
        }
        #[cfg(not(feature = "presentation"))]
        {
            true
        }
    }

    /// One frame's logic: every module's `before_tick`, the runner's
    /// `step(dt)`, then every module's `after_tick`.
    ///
    /// `run` calls this once per iteration. A caller driving its own loop
    /// calls it instead of `Runner::step`, so modules that hook either side
    /// of the tick — the extension host among them — still run.
    pub fn step(&mut self, dt: f32) {
        step_frame(&self.runner, &self.modules, dt);
    }

    /// Runs the `render` then `present` phases over every module, in
    /// registration order (design/modules.md).
    ///
    /// `run` calls this once per iteration; it is public so a caller
    /// driving `Runner::step` itself can complete a frame without
    /// reimplementing the phase order.
    pub fn present_frame(&self) {
        run_frame_phases(&self.modules);
    }

    /// Runs the complete, idempotent application shutdown sequence.
    pub fn shutdown(&mut self) {
        self.shutdown_from(ENGINE_SENDER);
    }

    pub(super) fn shutdown_from(&mut self, sender: &str) {
        if self.shutdown_started {
            return;
        }
        self.shutdown_started = true;
        run_shutdown(&self.runner, &self.modules, sender);
    }
}

/// One frame's presentation work: every module's `render`, then every
/// module's `present`, both in registration order.
///
/// Two full passes rather than render-then-present per module, because the
/// order across modules is what layers the frame — the renderer composites
/// its retained `gfx/*` batches in `render`, ui draws above that in its own
/// `render`, and only then does anything flip a buffer in `present`.
pub(super) fn run_frame_phases(modules: &[Arc<Mutex<Box<dyn Module>>>]) {
    for module in modules {
        module.lock().unwrap().render();
    }
    for module in modules {
        module.lock().unwrap().present();
    }
}

/// `before_tick`, the tick itself, and `after_tick`, over every module in
/// registration order.
pub(super) fn step_frame(runner: &Runner, modules: &[Arc<Mutex<Box<dyn Module>>>], dt: f32) {
    for module in modules {
        module.lock().unwrap().before_tick();
    }
    runner.step(dt);
    for module in modules {
        module.lock().unwrap().after_tick();
    }
}

pub(super) fn run_shutdown(runner: &Runner, modules: &[Arc<Mutex<Box<dyn Module>>>], sender: &str) {
    runner.begin_frame();
    runner.bus().publish(Envelope {
        topic: CloseRequested::TOPIC.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload: CloseRequested.encode(),
    });
    runner.bus().dispatch();
    for module in modules {
        module.lock().unwrap().begin_shutdown();
    }
    runner.bus().dispatch();
    for module in modules {
        module.lock().unwrap().shutdown();
    }
    runner.bus().dispatch();
}
