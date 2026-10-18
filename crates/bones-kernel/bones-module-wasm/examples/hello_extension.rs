//! Loads the `hello` WASM extension and drives it through a few ticks over
//! a real bus, proving the log line flows through the engine end to end.
//!
//! Build the extension first: cargo xtask example hello-demo
//! Then: cargo run -p bones-kernel-infra --example hello_extension

use bones_kernel_infra::bus::{BudgetLimits, Bus, EndpointBudget, Envelope, Registry};
use bones_kernel_infra::logging::Logger;
use bones_messages::tick::Tick;
use bones_messages::{EncodeMessage, Message};
use bones_module_wasm::host::{DisplayInfo, Host};

const HELLO_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../examples/extensions/hello-demo/target/wasm32-wasip2/release/hello_demo.wasm"
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
    let engine = bones_module_wasm::host::new_engine()?;
    let bus = Bus::new();

    let mut hello = Host::load(
        &engine,
        HELLO_WASM,
        "hello",
        bus.clone(),
        Registry::new(),
        Logger::default(),
        bones_kernel_infra::exit::ExitRequest::new(),
        DisplayInfo::default(),
        EndpointBudget::new(BudgetLimits::default()),
        bones_module_wasm::host::ExtensionTimeouts::default(),
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
