//! Extension host (architecture.md): loads one WASM component against the
//! contract (`wit/core.wit`) and calls its exports. Registers as an ordinary
//! bus endpoint — structure.md: modules and extensions are indistinguishable
//! on the bus. Topic subscriptions (including `core/tick`) are opt-in,
//! requested by the extension itself via the `subscribe` import during
//! `init` (messaging.md).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use bus::{Bus, Envelope, Handler, Registry};
use contract::bones::core::host_api::{Host as HostApiImports, Level, SendError};
use contract::Extension;
use logging::Logger;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store};

const TICK_TOPIC: &str = "core/tick";
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
        bus::SendError::Timeout => SendError::Timeout,
        bus::SendError::Cycle => SendError::Cycle,
    }
}

/// The only `Engine` configuration `Host` works with — component model
/// support is not optional here, so this avoids a caller forgetting it.
/// Epoch interruption (ADR-007's time budget) needs a ticker advancing the
/// shared epoch counter for `Store::set_epoch_deadline` to mean anything in
/// wall-clock terms; runs for the engine's lifetime, no shutdown needed for
/// a process-lifetime `Engine`.
pub fn new_engine() -> wasmtime::Result<Engine> {
    let engine = Engine::new(Config::new().wasm_component_model(true).epoch_interruption(true))?;
    let ticker = engine.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(EPOCH_TICK_INTERVAL);
        ticker.increment_epoch();
    });
    Ok(engine)
}

fn tick_dt(envelope: &Envelope) -> Option<f32> {
    if envelope.topic != TICK_TOPIC {
        return None;
    }
    let bytes: [u8; 4] = envelope.payload.as_slice().try_into().ok()?;
    Some(f32::from_le_bytes(bytes))
}

struct State {
    name: String,
    logger: Logger,
    bus: Bus,
    registry: Registry,
    wasi: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
    requested_topics: Vec<String>,
}

impl HostApiImports for State {
    fn log(&mut self, level: Level, message: String) {
        let category = "extension";
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

    /// TODO: `deadline_ms` is accepted but not enforced — dispatch is
    /// single-threaded, so a target is never actually busy long enough to
    /// wait out; only cycle detection can fail a send today (ADR-010).
    fn send(&mut self, endpoint: String, payload: Vec<u8>, _deadline_ms: u32) -> Result<Vec<u8>, SendError> {
        self.registry.call(&self.name, &endpoint, &payload).map_err(map_send_error)
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
    /// Loads `wasm_path`, links the `log`/`subscribe`/`publish`/`send`
    /// imports, and calls `init` once — under the same time budget as any
    /// other call, so a hanging `init` faults instead of blocking `load`
    /// forever. `name` is this extension's bus endpoint id — the `sender`
    /// on envelopes it publishes and the name `send` (ADR-010) reaches it
    /// by; `bus` is what `publish` reaches, `registry` is what `send`
    /// reaches.
    pub fn load(
        engine: &Engine,
        wasm_path: &str,
        name: &str,
        bus: Bus,
        registry: Registry,
        logger: Logger,
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
        match self.bindings.call_on_message(&mut *store, "", sender, payload) {
            Ok(reply) => reply,
            Err(err) => {
                store.data().logger.error("host", &format!("handler trapped during send: {err}"));
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
        let result = match tick_dt(envelope) {
            Some(dt) => self.bindings.call_on_tick(&mut *store, dt).map(|()| None),
            None => self.bindings.call_on_message(
                &mut *store,
                &envelope.topic,
                &envelope.sender,
                &envelope.payload,
            ),
        };
        if let Err(err) = result {
            store.data().logger.error("host", &format!("handler trapped: {err}"));
            self.faulted.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::RecordingSink;
    use std::sync::{Arc, Mutex};

    // Built by extensions/hello/build.ps1 (see its README).
    const HELLO_WASM: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/hello/target/wasm32-wasip2/release/hello.wasm"
    );
    // Built by extensions/runaway_demo/build.ps1 (see its README).
    const RUNAWAY_WASM: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/runaway_demo/target/wasm32-wasip2/release/runaway_demo.wasm"
    );

    fn load_hello(bus: Bus, logger: Logger) -> Host {
        let engine = new_engine().unwrap();
        Host::load(&engine, HELLO_WASM, "hello", bus, Registry::new(), logger)
            .expect("build extensions/hello first: pwsh extensions/hello/build.ps1")
    }

    fn test_state(name: &str, registry: Registry) -> State {
        State {
            name: name.to_string(),
            logger: Logger::default(),
            bus: Bus::new(),
            registry,
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
            table: wasmtime_wasi::ResourceTable::new(),
            requested_topics: Vec::new(),
        }
    }

    struct EchoRespond;
    impl bus::Respond for EchoRespond {
        fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
            Some([sender.as_bytes(), payload].concat())
        }
    }

    #[test]
    fn send_reaches_a_registered_target_and_returns_its_reply() {
        let registry = Registry::new();
        registry.insert("echo", Arc::new(EchoRespond));
        let mut state = test_state("caller", registry);

        let reply = state.send("echo".to_string(), b"hi".to_vec(), 1000).unwrap();

        assert_eq!(reply, b"callerhi");
    }

    #[test]
    fn send_to_an_unknown_endpoint_maps_to_the_wit_error() {
        let mut state = test_state("caller", Registry::new());

        assert_eq!(
            state.send("nobody".to_string(), Vec::new(), 1000),
            Err(SendError::UnknownEndpoint)
        );
    }

    #[test]
    fn send_to_self_maps_to_the_wit_cycle_error() {
        let registry = Registry::new();
        registry.insert("caller", Arc::new(EchoRespond));
        let mut state = test_state("caller", registry);

        assert_eq!(
            state.send("caller".to_string(), Vec::new(), 1000),
            Err(SendError::Cycle)
        );
    }

    #[test]
    fn respond_calls_on_message_with_an_empty_topic() {
        let sink = RecordingSink::new();
        let mut host = load_hello(Bus::new(), Logger::new(Arc::new(sink.clone())));

        host.respond("someone", b"data");

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("message on  from someone")),
            "expected on-message to run with an empty topic for a direct send, got {records:?}"
        );
    }

    #[test]
    fn init_logs_through_the_engine() {
        let sink = RecordingSink::new();
        let _host = load_hello(Bus::new(), Logger::new(Arc::new(sink.clone())));

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("init")),
            "expected an init log line, got {records:?}"
        );
    }

    #[test]
    fn requested_topics_reflects_what_init_subscribed_to() {
        let mut host = load_hello(Bus::new(), Logger::default());
        assert_eq!(host.requested_topics(), vec![TICK_TOPIC.to_string()]);
        assert!(host.requested_topics().is_empty(), "must drain, not repeat");
    }

    #[test]
    fn on_message_is_dispatched_for_non_tick_topics() {
        let sink = RecordingSink::new();
        let bus = Bus::new();
        let host = load_hello(bus.clone(), Logger::new(Arc::new(sink.clone())));

        let ep = bus.register("hello", host);
        ep.subscribe("test/event");

        bus.publish(Envelope {
            topic: "test/event".to_string(),
            sender: "someone".to_string(),
            correlation: None,
            payload: Vec::new(),
        });
        bus.dispatch();

        let records = sink.records();
        assert!(
            records
                .iter()
                .any(|(_, _, msg)| msg.contains("message on test/event from someone")),
            "expected an on-message log line, got {records:?}"
        );
    }

    #[test]
    fn on_tick_logs_through_the_engine_as_an_ordinary_bus_endpoint() {
        let sink = RecordingSink::new();
        let bus = Bus::new();
        let host = load_hello(bus.clone(), Logger::new(Arc::new(sink.clone())));

        let ep = bus.register("hello", host);
        ep.subscribe(TICK_TOPIC);

        bus.publish(Envelope {
            topic: TICK_TOPIC.to_string(),
            sender: "test".to_string(),
            correlation: None,
            payload: (1.0f32 / 60.0).to_le_bytes().to_vec(),
        });
        bus.dispatch();

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("tick")),
            "expected a tick log line, got {records:?}"
        );
    }

    #[test]
    fn publish_reaches_other_subscribers_on_the_same_bus() {
        let bus = Bus::new();
        let host = load_hello(bus.clone(), Logger::default());
        let hello_ep = bus.register("hello", host);
        hello_ep.subscribe("test/event");

        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = received.clone();
        let listener_ep = bus.register("listener", move |e: &Envelope| sink.lock().unwrap().push(e.clone()));
        listener_ep.subscribe("hello/received");

        bus.publish(Envelope {
            topic: "test/event".to_string(),
            sender: "someone".to_string(),
            correlation: None,
            payload: Vec::new(),
        });
        bus.dispatch(); // delivers test/event to hello; its publish() only enqueues (ADR-015)
        bus.dispatch(); // delivers hello/received to listener

        let got = received.lock().unwrap();
        assert!(
            got.iter().any(|e| e.topic == "hello/received" && e.sender == "hello"),
            "expected hello's publish to reach another subscriber, got {got:?}"
        );
    }

    fn tick_envelope() -> Envelope {
        Envelope {
            topic: TICK_TOPIC.to_string(),
            sender: "test".to_string(),
            correlation: None,
            payload: (1.0f32 / 60.0).to_le_bytes().to_vec(),
        }
    }

    #[test]
    fn a_call_that_never_returns_traps_and_faults_instead_of_hanging_forever() {
        let sink = RecordingSink::new();
        let engine = new_engine().unwrap();
        let mut host = Host::load(&engine, RUNAWAY_WASM, "runaway", Bus::new(), Registry::new(), Logger::new(Arc::new(sink.clone())))
            .expect("build extensions/runaway_demo first: pwsh extensions/runaway_demo/build.ps1");
        assert!(!host.is_faulted(), "must not start out faulted");

        host.handle(&tick_envelope()); // blocks for ~the time budget, then traps

        assert!(host.is_faulted(), "an on-tick that never returns must fault the extension");
        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("handler trapped")),
            "expected a trap log line, got {records:?}"
        );
    }

    #[test]
    fn a_faulted_host_ignores_further_deliveries_instead_of_hanging_again() {
        let engine = new_engine().unwrap();
        let mut host = Host::load(&engine, RUNAWAY_WASM, "runaway", Bus::new(), Registry::new(), Logger::default())
            .expect("build extensions/runaway_demo first: pwsh extensions/runaway_demo/build.ps1");
        host.handle(&tick_envelope());
        assert!(host.is_faulted());

        // If this didn't short-circuit on is_faulted(), it would hang for
        // another full time budget (or forever, without epoch interruption
        // active a second time) instead of returning immediately.
        host.handle(&tick_envelope());
    }
}
