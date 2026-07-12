//! Loads the `hello` WASM extension and drives it through a few ticks over
//! a real bus, proving the log line flows through the engine end to end.
//!
//! Build the extension first: pwsh extensions/hello/build.ps1
//! Then: cargo run -p host --example hello_extension

use bus::{Bus, Envelope};
use host::Host;
use logging::Logger;

const HELLO_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../extensions/hello/target/wasm32-wasip2/release/hello.wasm"
);
const TICK_TOPIC: &str = "core/tick";

fn tick(bus: &Bus, dt: f32) {
    bus.publish(Envelope {
        topic: TICK_TOPIC.to_string(),
        sender: "demo".to_string(),
        correlation: None,
        payload: dt.to_le_bytes().to_vec(),
    });
    bus.dispatch();
}

fn main() -> wasmtime::Result<()> {
    let engine = host::new_engine()?;
    let bus = Bus::new();

    let mut hello = Host::load(&engine, HELLO_WASM, "hello", bus.clone(), Logger::default())?;
    let topics = hello.requested_topics();
    let ep = bus.register("hello", hello);
    for topic in &topics {
        ep.subscribe(topic);
    }

    println!("-- running 3 ticks --");
    for _ in 0..3 {
        tick(&bus, 1.0 / 60.0);
    }

    Ok(())
}
