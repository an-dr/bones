//! Loads the `hello` WASM extension and drives it through a few ticks over
//! a real bus, proving the log line flows through the engine end to end.
//!
//! Build the extension first: pwsh crates/bones-extension-hello/build.ps1
//! Then: cargo run -p bones-kernel --example hello_extension

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bones_kernel::bus::{BudgetLimits, Bus, EndpointBudget, Envelope, Registry};
use bones_kernel::logging::Logger;
use bones_kernel::wasm_extensions::host::{DisplayInfo, Host};
use bones_messages::tick::Tick;
use bones_messages::{EncodeMessage, Message};

const HELLO_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../crates/bones-extension-hello/target/wasm32-wasip2/release/bones_extension_hello.wasm"
);

fn tick_once(bus: &Bus, dt: f32) {
    let tick = Tick { dt };
    bus.publish(Envelope {
        topic: Tick::TOPIC.to_string(),
        sender: "demo".to_string(),
        correlation: None,
        payload: tick.encode(),
    });
    bus.dispatch();
}

fn main() -> wasmtime::Result<()> {
    let engine = bones_kernel::wasm_extensions::host::new_engine()?;
    let bus = Bus::new();

    let mut hello = Host::load(
        &engine,
        HELLO_WASM,
        "hello",
        bus.clone(),
        Registry::new(),
        Logger::default(),
        Arc::new(AtomicBool::new(false)),
        DisplayInfo::default(),
        EndpointBudget::new(BudgetLimits::default()),
        bones_kernel::wasm_extensions::host::ExtensionTimeouts::default(),
    )?;
    let topics = hello.requested_topics();
    let ep = bus.register("hello", hello);
    for topic in &topics {
        ep.subscribe(topic);
    }

    println!("-- running 3 ticks --");
    for _ in 0..3 {
        tick_once(&bus, 1.0 / 60.0);
    }

    Ok(())
}
