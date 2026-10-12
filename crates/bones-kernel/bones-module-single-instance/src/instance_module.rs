use crate::{Handoff, Primary};
use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_kernel_infra::logging::Logger;
use bones_kernel_infra::window_control::WindowControl;
use bones_messages::instance::{Launched, ENDPOINT};
use bones_messages::Message;

/// Publishes each later launch and raises the window.
///
/// - Drains the claim's channel once per frame, in `render`, without
///   blocking: a launch costs the frame nothing when none arrived.
/// - Raises through `window-control` when the composition has a window; a
///   headless composition still publishes.
pub struct InstanceModule {
    primary: Primary,
    logger: Logger,
    bus: Option<Bus>,
    window: Option<WindowControl>,
}

impl InstanceModule {
    /// The module for the claim `primary` holds.
    pub fn new(primary: Primary, logger: Logger) -> Self {
        Self {
            primary,
            logger,
            bus: None,
            window: None,
        }
    }

    fn publish(&self, bus: &Bus, handoff: &Handoff) {
        let cwd = handoff.cwd.to_string_lossy();
        let launched = Launched {
            args: handoff.args.iter().map(String::as_str).collect(),
            cwd: &cwd,
        };
        let Some(payload) = launched.try_encode() else {
            self.logger.warn(
                ENDPOINT,
                "dropped a launch whose arguments exceed what instance/launched can carry",
            );
            return;
        };
        bus.publish(Envelope {
            topic: Launched::TOPIC.into(),
            sender: ENDPOINT.into(),
            correlation: None,
            payload,
        });
    }
}

impl Handler for InstanceModule {
    fn handle(&mut self, _envelope: &Envelope) {}
}

impl Module for InstanceModule {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        self.bus = context.get_service::<Bus>().cloned();
        self.window = context.get_service::<WindowControl>().cloned();
        self.bus
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "no Bus service available".into())
    }

    fn render(&mut self) {
        let Some(bus) = self.bus.clone() else { return };
        let mut launched = false;
        while let Ok(handoff) = self.primary.incoming().try_recv() {
            self.publish(&bus, &handoff);
            launched = true;
        }
        if !launched {
            return;
        }
        if let Some(window) = &self.window {
            if let Err(error) = window.raise() {
                self.logger
                    .warn(ENDPOINT, &format!("could not raise the window: {error}"));
            }
        }
    }
}

#[cfg(test)]
mod tests;
