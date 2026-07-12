//! Extension host (architecture.md): loads one WASM component against the
//! contract (`wit/core.wit`) and calls its exports. Registers as an ordinary
//! bus endpoint — structure.md: modules and extensions are indistinguishable
//! on the bus. Topic subscriptions (including `core/tick`) are opt-in,
//! requested by the extension itself via the `subscribe` import during
//! `init` (messaging.md).

use std::sync::Mutex;

use bus::{Bus, Envelope, Handler};
use contract::bones::core::host_api::{Host as HostApiImports, Level};
use contract::Extension;
use logging::Logger;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store};

const TICK_TOPIC: &str = "core/tick";

/// The only `Engine` configuration `Host` works with — component model
/// support is not optional here, so this avoids a caller forgetting it.
pub fn new_engine() -> wasmtime::Result<Engine> {
    Engine::new(Config::new().wasm_component_model(true))
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
}

impl Host {
    /// Loads `wasm_path`, links the `log`/`subscribe`/`publish` imports, and
    /// calls `init` once. `name` is this extension's bus endpoint id — the
    /// `sender` on envelopes it publishes; `bus` is what `publish` reaches.
    pub fn load(
        engine: &Engine,
        wasm_path: &str,
        name: &str,
        bus: Bus,
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
                wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
                table: wasmtime_wasi::ResourceTable::new(),
                requested_topics: Vec::new(),
            },
        );
        let bindings = Extension::instantiate(&mut store, &component, &linker)?;
        bindings.call_init(&mut store)?;

        Ok(Self {
            store: Mutex::new(store),
            bindings,
        })
    }

    /// Topics the extension asked for via `subscribe` during `init`. TODO:
    /// `init` is the only opportunity to subscribe today — no way to
    /// subscribe again later. Drains the list; meant to be read once, right
    /// after `load`, by whoever registers this `Host` on the bus.
    pub fn requested_topics(&mut self) -> Vec<String> {
        std::mem::take(&mut self.store.get_mut().unwrap().data_mut().requested_topics)
    }
}

impl Handler for Host {
    /// `&mut self` already gives exclusive access (the bus never calls a
    /// handler concurrently, ADR-013), so the inner `Mutex` never contends.
    fn handle(&mut self, envelope: &Envelope) {
        let store = self.store.get_mut().unwrap();
        let result = match tick_dt(envelope) {
            Some(dt) => self.bindings.call_on_tick(&mut *store, dt),
            None => self.bindings.call_on_message(
                &mut *store,
                &envelope.topic,
                &envelope.sender,
                &envelope.payload,
            ),
        };
        if let Err(err) = result {
            store.data().logger.error("host", &format!("handler trapped: {err}"));
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

    fn load_hello(bus: Bus, logger: Logger) -> Host {
        let engine = new_engine().unwrap();
        Host::load(&engine, HELLO_WASM, "hello", bus, logger)
            .expect("build extensions/hello first: pwsh extensions/hello/build.ps1")
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
}
