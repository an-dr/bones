use bones_messages::tick::Tick;
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use bus::{Bus, Envelope};
use logging::Logger;

mod engine;
mod loading;
mod supervisor;
pub use engine::{BuiltEngine, Engine};
pub use supervisor::Supervisor;

/// Reads `dt` back out of a `core/tick` envelope, `None` if the topic
/// doesn't match or the payload isn't 4 LE bytes.
pub fn read_tick_dt(envelope: &Envelope) -> Option<f32> {
    if envelope.topic != Tick::TOPIC {
        return None;
    }
    Tick::decode(&envelope.payload).ok().map(|tick| tick.dt)
}

/// Headless frame-phase loop skeleton (ADR-014). Bounded and step-driven;
/// tick is an ordinary `core/tick` bus message, not a separate mechanism.
pub struct Runner {
    bus: Bus,
    logger: Logger,
}

impl Runner {
    pub fn new(bus: Bus, logger: Logger) -> Self {
        Self { bus, logger }
    }

    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// One frame: publish `core/tick` with the given (caller-supplied,
    /// virtual) dt, then dispatch everything queued.
    pub fn step(&self, dt: f32) {
        self.logger.debug("runner", &format!("tick dt={dt}"));
        let tick = Tick { dt };
        self.bus.publish(Envelope {
            topic: Tick::TOPIC.to_string(),
            sender: "runner".to_string(),
            correlation: None,
            payload: tick.encode(),
        });
        self.bus.dispatch();
    }

    pub fn run_for(&self, ticks: u32, dt: f32) {
        for _ in 0..ticks {
            self.step(dt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::RecordingSink;
    use std::sync::{Arc, Mutex};

    struct CountingEndpoint {
        ticks: Arc<Mutex<Vec<f32>>>,
    }

    impl bus::Handler for CountingEndpoint {
        fn handle(&mut self, envelope: &Envelope) {
            if let Some(dt) = read_tick_dt(envelope) {
                self.ticks.lock().unwrap().push(dt);
            }
        }
    }

    #[test]
    fn step_delivers_tick_to_a_subscribed_endpoint() {
        let bus = Bus::new();
        let ticks = Arc::new(Mutex::new(Vec::new()));
        let ep = bus.register(
            "level",
            CountingEndpoint {
                ticks: ticks.clone(),
            },
        );
        ep.subscribe(Tick::TOPIC);

        let runner = Runner::new(bus, Logger::default());
        runner.step(0.016);

        assert_eq!(*ticks.lock().unwrap(), vec![0.016]);
    }

    #[test]
    fn run_for_delivers_ticks_in_order() {
        let bus = Bus::new();
        let ticks = Arc::new(Mutex::new(Vec::new()));
        let ep = bus.register(
            "level",
            CountingEndpoint {
                ticks: ticks.clone(),
            },
        );
        ep.subscribe(Tick::TOPIC);

        let runner = Runner::new(bus, Logger::default());
        runner.run_for(3, 0.016);

        assert_eq!(*ticks.lock().unwrap(), vec![0.016, 0.016, 0.016]);
    }

    #[test]
    fn an_endpoint_not_subscribed_to_tick_receives_nothing() {
        let bus = Bus::new();
        let ticks = Arc::new(Mutex::new(Vec::new()));
        let ep = bus.register(
            "ui",
            CountingEndpoint {
                ticks: ticks.clone(),
            },
        );
        ep.subscribe("ui/*");

        let runner = Runner::new(bus, Logger::default());
        runner.step(0.016);

        assert!(ticks.lock().unwrap().is_empty());
    }

    #[test]
    fn each_step_logs_a_debug_line() {
        let sink = RecordingSink::new();
        let runner = Runner::new(Bus::new(), Logger::new(Arc::new(sink.clone())));

        runner.run_for(2, 0.016);

        assert_eq!(sink.records().len(), 2);
    }
}
