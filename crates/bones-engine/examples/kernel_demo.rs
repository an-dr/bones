//! A tick loop over native test endpoints, showing topic routing and FIFO
//! delivery order (ADR-009).
//! Run with: cargo run -p runner --example kernel_demo

use bones_messages::tick::Tick;
use bones_messages::Message;
use bones_engine::bus::{Bus, Envelope, Handler};
use bones_engine::logging::Logger;
use bones_engine::Runner;

struct Printer {
    name: &'static str,
}

impl Handler for Printer {
    fn handle(&mut self, envelope: &Envelope) {
        match bones_engine::read_tick_dt(envelope) {
            Some(dt) => println!("[{}] core/tick dt={dt}", self.name),
            None => println!(
                "[{}] {} <- {:?}",
                self.name,
                envelope.topic,
                String::from_utf8_lossy(&envelope.payload)
            ),
        }
    }
}

fn envelope(topic: &str, payload: &str) -> Envelope {
    Envelope {
        topic: topic.to_string(),
        sender: "world".to_string(),
        correlation: None,
        payload: payload.as_bytes().to_vec(),
    }
}

fn main() {
    let bus = Bus::new();

    let level = bus.register("level", Printer { name: "level" });
    level.subscribe(Tick::TOPIC);
    level.subscribe("game/*");

    let ui = bus.register("ui", Printer { name: "ui" });
    ui.subscribe("ui/*");

    // Queued before any dispatch happens -- delivered in this exact order
    // on the first tick, proving per-sender FIFO (ADR-009).
    bus.publish(envelope("game/spawn", "goblin"));
    bus.publish(envelope("game/spawn", "slime"));
    bus.publish(envelope("ui/toast", "Welcome!"));

    let runner = Runner::new(bus, Logger::default());

    println!("-- running 3 ticks --");
    runner.run_for(3, 1.0 / 60.0);
}
