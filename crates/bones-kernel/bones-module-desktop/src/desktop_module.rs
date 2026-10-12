use std::sync::Arc;

use crate::{Integration, Locations};
use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_messages::desktop::{Action, Outcome, Request, Result as DesktopResult, ENDPOINT};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

/// `desktop/request` in, `desktop/result` out, for one application.
///
/// Each request runs on a thread of its own: registration writes files,
/// registry keys, and shortcuts, and on Linux refreshes desktop caches, none
/// of which may stall a frame.
pub struct DesktopModule {
    integration: Arc<Integration<'static>>,
    /// `None` when the current user's locations could not be resolved; every
    /// request then answers with that failure rather than going unanswered.
    locations: Option<Arc<Locations>>,
    bus: Option<Bus>,
}

impl DesktopModule {
    /// Registers `integration` in the current user's own locations.
    pub fn new(integration: Integration<'static>) -> Self {
        Self::with_locations(integration, Locations::for_current_user())
    }

    /// Registers `integration` in `locations` — scratch ones, in a test.
    pub fn with_locations(integration: Integration<'static>, locations: Option<Locations>) -> Self {
        Self {
            integration: Arc::new(integration),
            locations: locations.map(Arc::new),
            bus: None,
        }
    }

    fn start(&self, request: Request) {
        let Some(bus) = self.bus.clone() else { return };
        let integration = Arc::clone(&self.integration);
        let locations = self.locations.clone();
        std::thread::spawn(move || {
            let outcome = match &locations {
                Some(locations) => perform(request.action, &integration, locations),
                None => Err("desktop registration is not available on this system".into()),
            };
            let outcome = match &outcome {
                Ok(registered) => Outcome::Registered(*registered),
                Err(reason) => Outcome::Failed(reason),
            };
            bus.publish(Envelope {
                topic: DesktopResult::TOPIC.into(),
                sender: ENDPOINT.into(),
                correlation: Some(u64::from(request.request_id)),
                payload: DesktopResult {
                    request_id: request.request_id,
                    outcome,
                }
                .encode(),
            });
        });
    }
}

/// Runs one action and reports whether the application is registered after
/// it.
fn perform(
    action: Action,
    integration: &Integration,
    locations: &Locations,
) -> Result<bool, String> {
    match action {
        Action::Register => {
            // A registration pointing at a missing executable would start
            // nothing; refusing says why instead.
            if !integration.executable.is_file() {
                return Err(format!(
                    "{} is not installed at {}",
                    integration.name,
                    integration.executable.display()
                ));
            }
            crate::register(integration, locations)?;
        }
        Action::Unregister => crate::unregister(integration, locations)?,
        Action::Status => {}
    }
    Ok(crate::is_registered(integration, locations))
}

impl Handler for DesktopModule {
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic != Request::TOPIC {
            return;
        }
        if let Ok(request) = Request::decode(&envelope.payload) {
            self.start(request);
        }
    }
}

impl Module for DesktopModule {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        context.subscribe(Request::TOPIC);
        self.bus = context.get_service::<Bus>().cloned();
        self.bus
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "no Bus service available".into())
    }
}

#[cfg(test)]
mod tests;
