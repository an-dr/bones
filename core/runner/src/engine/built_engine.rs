use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use bus::Module;
use renderer::Renderer;
use ui::Ui;

use crate::Runner;
use crate::Supervisor;

/// Everything `Engine::build` wires up: the step-driven `Runner`, the
/// platform window (if `.window(...)` was set), the renderer (if
/// `.renderer()` was set), the ui module (if `.ui()` was set), every
/// `.module(...)`-injected native module, and the `Supervisor` sweeping
/// loaded extensions for faults and file changes.
pub struct BuiltEngine {
    pub runner: Runner,
    pub platform: Option<platform::Platform>,
    pub renderer: Option<Arc<Mutex<Renderer>>>,
    pub ui: Option<Arc<Mutex<Ui>>>,
    pub modules: Vec<Arc<Mutex<Box<dyn Module>>>>,
    pub supervisor: Supervisor,
    /// Set by any loaded extension's `request-exit` host-api call; `run`'s
    /// loop breaks once true, the same as `Platform::quit_requested()`.
    pub exit_requested: Arc<AtomicBool>,
}
