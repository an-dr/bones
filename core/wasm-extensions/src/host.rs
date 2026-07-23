//! Extension host (architecture.md): loads one WASM component against the
//! contract (`wit/core.wit`) and calls its exports. Registers as an ordinary
//! bus endpoint — structure.md: modules and extensions are indistinguishable
//! on the bus. Topic subscriptions (including `core/tick`) are opt-in,
//! requested by the extension itself via the `subscribe` import during
//! `init` (messaging.md).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bones_messages::tick::Tick;
use bones_messages::{DecodeMessage, Message};
use bus::{Bus, Envelope, Handler, Registry};
use contract::bones::core::host_api::{DisplayMode, Host as HostApiImports, Level, SendError};
use contract::Extension;
use logging::Logger;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store};

/// How often the background thread `new_engine` spawns advances the shared
/// epoch counter every loaded extension's calls are budgeted against.
const EPOCH_TICK_INTERVAL: Duration = Duration::from_millis(5);
/// Per-call timeout (ADR-007's time budget), in epoch ticks: 10 * 5ms =
/// 50ms. A call that hasn't returned by then traps, faulting the extension.
const CALL_TIMEOUT_TICKS: u64 = 10;
/// Timeout for `instantiate` + `init` (200 ticks * 5ms = 1s): cold JIT
/// compilation is a legitimate one-time cost `CALL_TIMEOUT_TICKS` isn't
/// meant to cover.
const LOAD_TIMEOUT_TICKS: u64 = 200;

fn map_send_error(err: bus::SendError) -> SendError {
    match err {
        bus::SendError::UnknownEndpoint => SendError::UnknownEndpoint,
        bus::SendError::Cycle => SendError::Cycle,
        // TODO: the WIT contract has no timeout case yet; `bus::SendError`
        // grew one ahead of it. `unreachable!` rather than silently mapping
        // to a wrong variant — bus.rs's own doc comment says dispatch is
        // single-threaded, so nothing constructs this today; the panic is
        // the signal to add a real WIT case the day something does.
        bus::SendError::Timeout => unreachable!("bus::SendError::Timeout is not constructed yet"),
    }
}

/// The only `Engine` configuration `Host` works with — component model
/// support is not optional here, so this avoids a caller forgetting it.
/// Epoch interruption (ADR-007's time budget) needs a ticker advancing the
/// shared epoch counter for `Store::set_epoch_deadline` to mean anything in
/// wall-clock terms; runs for the engine's lifetime, no shutdown needed for
/// a process-lifetime `Engine`.
pub fn new_engine() -> wasmtime::Result<Engine> {
    let engine = Engine::new(
        Config::new()
            .wasm_component_model(true)
            .epoch_interruption(true),
    )?;
    let ticker = engine.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(EPOCH_TICK_INTERVAL);
        ticker.increment_epoch();
    });
    Ok(engine)
}

/// Display capability info queried once by `platform::Platform` (before any
/// window hand-off) and threaded into every `Host::load` call — small,
/// read-only, static for the process's lifetime, so a plain owned copy per
/// `Host` is simpler than `exit_requested`'s shared-mutable-flag `Arc`.
/// Bundled into one struct rather than two more `Host::load` parameters, to
/// stay clear of `clippy::too_many_arguments`.
#[derive(Debug, Clone, Default)]
pub struct DisplayInfo {
    pub modes: Vec<(u32, u32)>,
    pub native: Option<(u32, u32)>,
}

fn read_tick_dt(envelope: &Envelope) -> Option<f32> {
    if envelope.topic != Tick::TOPIC {
        return None;
    }
    Tick::decode(&envelope.payload).ok().map(|tick| tick.dt)
}

// `State` and `Host` stay in one file rather than splitting further: `State`
// is purely `Host`'s internal store data (implements the WIT imports and
// `WasiView` only for `Host`'s own use), never meaningful on its own.
struct State {
    name: String,
    logger: Logger,
    bus: Bus,
    registry: Registry,
    wasi: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
    requested_topics: Vec<String>,
    /// Shared with whoever runs the engine's own loop (`runner::Engine::run`)
    /// — set here, read there. Not `Host`'s to act on: closing the app is
    /// the run loop's job, this only signals the request.
    exit_requested: Arc<AtomicBool>,
    display_info: DisplayInfo,
}

impl HostApiImports for State {
    fn log(&mut self, level: Level, message: String) {
        let category = self.name.as_str();
        match level {
            Level::Debug => self.logger.debug(category, &message),
            Level::Info => self.logger.info(category, &message),
            Level::Warn => self.logger.warn(category, &message),
            Level::Error => self.logger.error(category, &message),
        }
    }

    fn subscribe(&mut self, topic: String) {
        self.requested_topics.push(topic);
    }

    fn publish(&mut self, topic: String, payload: Vec<u8>) {
        self.bus.publish(Envelope {
            topic,
            sender: self.name.clone(),
            correlation: None,
            payload,
        });
    }

    fn send(&mut self, endpoint: String, payload: Vec<u8>) -> Result<Vec<u8>, SendError> {
        self.registry
            .call(&self.name, &endpoint, &payload)
            .map_err(map_send_error)
    }

    fn request_exit(&mut self) {
        self.exit_requested.store(true, Ordering::Relaxed);
    }

    fn list_display_modes(&mut self) -> Vec<DisplayMode> {
        self.display_info
            .modes
            .iter()
            .map(|&(width, height)| DisplayMode { width, height })
            .collect()
    }

    fn native_display_mode(&mut self) -> Option<DisplayMode> {
        self.display_info
            .native
            .map(|(width, height)| DisplayMode { width, height })
    }
}

// wasm32-wasip2 components always import some WASI Preview 2 interfaces
// (poll, clocks, ...) via Rust's std runtime, even with no direct WASI use
// in the guest — deny-by-default context, no filesystem/network/stdio.
impl wasmtime_wasi::WasiView for State {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

pub struct Host {
    // wasmtime's ResourceTable (needed for the WASI plumbing above) holds
    // `Box<dyn Any + Send>`, not `Sync` — the Mutex is what makes `Host`
    // satisfy `Handler: Send + Sync`, not extra serialization (the bus
    // already serializes per-endpoint calls, ADR-013).
    store: Mutex<Store<State>>,
    bindings: Extension,
    /// Set once a call traps or exceeds its time budget (ADR-007) and never
    /// cleared — a faulted `Host` is done; quarantining it (dropping this
    /// instance, releasing its registrations) is whoever holds it's job,
    /// not this flag's.
    faulted: AtomicBool,
}

impl Host {
    /// Loads `wasm_path`, links the `log`/`subscribe`/`publish`/`send`/
    /// `request-exit`/`list-display-modes`/`native-display-mode` imports,
    /// and calls `init` once — under the same time budget as any other
    /// call, so a hanging `init` faults instead of blocking `load` forever.
    /// `name` is this extension's bus endpoint id — the `sender` on
    /// envelopes it publishes and the name `send` (ADR-010) reaches it by;
    /// `bus` is what `publish` reaches, `registry` is what `send` reaches,
    /// `exit_requested` is what `request-exit` sets (the caller's own clone
    /// is how it later reads that request), `display_info` backs the two
    /// display-mode query imports.
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        engine: &Engine,
        wasm_path: &str,
        name: &str,
        bus: Bus,
        registry: Registry,
        logger: Logger,
        exit_requested: Arc<AtomicBool>,
        display_info: DisplayInfo,
    ) -> wasmtime::Result<Self> {
        let component = Component::from_file(engine, wasm_path)?;
        let mut linker = Linker::new(engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        Extension::add_to_linker::<_, HasSelf<_>>(&mut linker, |state: &mut State| state)?;

        let mut store = Store::new(
            engine,
            State {
                name: name.to_string(),
                logger,
                bus,
                registry,
                wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
                table: wasmtime_wasi::ResourceTable::new(),
                requested_topics: Vec::new(),
                exit_requested,
                display_info,
            },
        );
        // Epoch interruption traps immediately on any check once enabled
        // (Config::epoch_interruption) until a deadline is set — must be
        // set before instantiate, which can itself run guest code.
        store.set_epoch_deadline(LOAD_TIMEOUT_TICKS);
        let bindings = Extension::instantiate(&mut store, &component, &linker)?;
        bindings.call_init(&mut store)?;

        Ok(Self {
            store: Mutex::new(store),
            bindings,
            faulted: AtomicBool::new(false),
        })
    }

    /// Topics the extension asked for via `subscribe` during `init`. TODO:
    /// `init` is the only opportunity to subscribe today — no way to
    /// subscribe again later. Drains the list; meant to be read once, right
    /// after `load`, by whoever registers this `Host` on the bus.
    pub fn requested_topics(&mut self) -> Vec<String> {
        std::mem::take(&mut self.store.get_mut().unwrap().data_mut().requested_topics)
    }

    /// Whether a call has ever trapped or exceeded its time budget
    /// (ADR-007). Sticky — checked before every later call so a faulted
    /// extension is never called into again.
    pub fn is_faulted(&self) -> bool {
        self.faulted.load(Ordering::Relaxed)
    }

    /// Answers a direct `send` targeting this extension (ADR-010,
    /// `bus::Respond`): calls `on-message` with an empty topic — direct
    /// messages have none (messaging.md) — and returns its reply. Same
    /// timeout/fault handling as `Handler::handle`: a no-op once faulted,
    /// and a hang here faults the extension the same way a hang in
    /// `on-tick`/`on-message` would.
    pub fn respond(&mut self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        if self.is_faulted() {
            return None;
        }
        let store = self.store.get_mut().unwrap();
        store.set_epoch_deadline(CALL_TIMEOUT_TICKS);
        match self
            .bindings
            .call_on_message(&mut *store, "", sender, payload)
        {
            Ok(reply) => reply,
            Err(err) => {
                store
                    .data()
                    .logger
                    .error("host", &format!("handler trapped during send: {err}"));
                self.faulted.store(true, Ordering::Relaxed);
                None
            }
        }
    }
}

impl Handler for Host {
    /// `&mut self` already gives exclusive access (the bus never calls a
    /// handler concurrently, ADR-013), so the inner `Mutex` never contends.
    /// A no-op once faulted — quarantine (dropping/unregistering this
    /// instance) is the caller's job, not this method's; until that
    /// happens, silently ignoring further deliveries is the safe default.
    /// Any reply `on-message` returns is ignored here too — pub/sub
    /// delivery has nowhere to send it back to; only a direct `send`
    /// (`respond`) reads it.
    fn handle(&mut self, envelope: &Envelope) {
        if self.is_faulted() {
            return;
        }
        let store = self.store.get_mut().unwrap();
        store.set_epoch_deadline(CALL_TIMEOUT_TICKS);
        let result = match read_tick_dt(envelope) {
            Some(dt) => self.bindings.call_on_tick(&mut *store, dt).map(|()| None),
            None => self.bindings.call_on_message(
                &mut *store,
                &envelope.topic,
                &envelope.sender,
                &envelope.payload,
            ),
        };
        if let Err(err) = result {
            store
                .data()
                .logger
                .error("host", &format!("handler trapped: {err}"));
            self.faulted.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests;
