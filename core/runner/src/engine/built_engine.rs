use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use bones_messages::window::CloseRequested;
use bones_messages::{EncodeMessage, Message};
use bus::{Envelope, Module};
use renderer::Renderer;
use ui::Ui;

use crate::loading::ENGINE_SENDER;
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
    pub(super) shutdown_started: bool,
}

impl BuiltEngine {
    /// Runs the complete, idempotent application shutdown sequence.
    pub fn shutdown(&mut self) {
        self.shutdown_from(ENGINE_SENDER);
    }

    pub(super) fn shutdown_from(&mut self, sender: &str) {
        if self.shutdown_started {
            return;
        }
        self.shutdown_started = true;
        run_shutdown(&self.runner, &mut self.supervisor, &self.modules, sender);
    }
}

pub(super) fn run_shutdown(
    runner: &Runner,
    supervisor: &mut Supervisor,
    modules: &[Arc<Mutex<Box<dyn Module>>>],
    sender: &str,
) {
    runner.bus().publish(Envelope {
        topic: CloseRequested::TOPIC.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload: CloseRequested.encode(),
    });
    runner.bus().dispatch();
    supervisor.check();

    supervisor.shutdown_all();
    runner.bus().dispatch();
    for module in modules {
        module.lock().unwrap().shutdown();
    }
    runner.bus().dispatch();
}
