//! Extension host (architecture.md): loads one WASM component against the
//! minimal contract (`wit/core.wit`) and calls its exports. Registers as an
//! ordinary bus endpoint — structure.md: modules and extensions are
//! indistinguishable on the bus — subscribed to `core/tick`.

use std::sync::Mutex;

use bus::{Envelope, Handler};
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
    logger: Logger,
    wasi: wasmtime_wasi::WasiCtx,
    table: wasmtime_wasi::ResourceTable,
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
    /// Loads `wasm_path`, links the `log` import, and calls `init` once.
    pub fn load(engine: &Engine, wasm_path: &str, logger: Logger) -> wasmtime::Result<Self> {
        let component = Component::from_file(engine, wasm_path)?;
        let mut linker = Linker::new(engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        Extension::add_to_linker::<_, HasSelf<_>>(&mut linker, |state: &mut State| state)?;

        let mut store = Store::new(
            engine,
            State {
                logger,
                wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
                table: wasmtime_wasi::ResourceTable::new(),
            },
        );
        let bindings = Extension::instantiate(&mut store, &component, &linker)?;
        bindings.call_init(&mut store)?;

        Ok(Self {
            store: Mutex::new(store),
            bindings,
        })
    }
}

impl Handler for Host {
    /// Only `core/tick` matters at this rung — no `on-message` export yet.
    /// `&mut self` already gives exclusive access (the bus never calls a
    /// handler concurrently, ADR-013), so this never contends.
    fn handle(&mut self, envelope: &Envelope) {
        let Some(dt) = tick_dt(envelope) else {
            return;
        };
        let store = self.store.get_mut().unwrap();
        if let Err(err) = self.bindings.call_on_tick(&mut *store, dt) {
            store.data().logger.error("host", &format!("on-tick trapped: {err}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus::Bus;
    use logging::RecordingSink;
    use std::sync::Arc;

    // Built by extensions/hello/build.ps1 (see its README).
    const HELLO_WASM: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/hello/target/wasm32-wasip2/release/hello.wasm"
    );

    fn load_hello(logger: Logger) -> Host {
        let engine = new_engine().unwrap();
        Host::load(&engine, HELLO_WASM, logger)
            .expect("build extensions/hello first: pwsh extensions/hello/build.ps1")
    }

    #[test]
    fn init_logs_through_the_engine() {
        let sink = RecordingSink::new();
        let _host = load_hello(Logger::new(Arc::new(sink.clone())));

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("init")),
            "expected an init log line, got {records:?}"
        );
    }

    #[test]
    fn on_tick_logs_through_the_engine_as_an_ordinary_bus_endpoint() {
        let sink = RecordingSink::new();
        let host = load_hello(Logger::new(Arc::new(sink.clone())));

        let bus = Bus::new();
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
}
