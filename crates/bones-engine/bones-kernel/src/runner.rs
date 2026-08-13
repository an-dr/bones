use crate::bus::{Bus, Envelope};
use crate::logging::Logger;
use bones_messages::tick::Tick;
use bones_messages::{DecodeMessage, EncodeMessage, Message};

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

    /// One frame: restore extension message allowances, publish `core/tick`
    /// with the given (caller-supplied, virtual) dt, then dispatch everything
    /// queued.
    pub fn step(&self, dt: f32) {
        self.begin_frame();
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

    /// Starts a frame without publishing a tick, for nonstandard drivers
    /// and the orderly shutdown phase.
    pub fn begin_frame(&self) {
        self.bus.begin_frame();
    }

    pub fn run_for(&self, ticks: u32, dt: f32) {
        for _ in 0..ticks {
            self.step(dt);
        }
    }
}

#[cfg(test)]
mod tests;
