use std::collections::HashSet;

use bones_kernel::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_kernel::logging::Logger;
use bones_messages::lifecycle::{Event, LifecycleEvent};
use bones_messages::web::{
    ClosePanel, Command, Navigate, PageMessage, PanelClosed, PanelFailed, PanelOpened, SendJson,
    ENDPOINT,
};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

use crate::{Backend, BackendEvent};

/// Owns the web protocol independently of a concrete browser runtime.
pub struct Web {
    bus: Bus,
    logger: Logger,
    backend: Box<dyn Backend>,
    panels: HashSet<(String, String)>,
}

impl Web {
    pub fn new(bus: Bus, logger: Logger, backend: impl Backend + 'static) -> Self {
        Self {
            bus,
            logger,
            backend: Box::new(backend),
            panels: HashSet::new(),
        }
    }

    fn open(&mut self, owner: &str, open: bones_messages::web::OpenPanel<'_>) {
        let key = (owner.to_string(), open.panel.to_string());
        if self.panels.contains(&key) {
            self.failed(owner, open.panel, "panel already exists");
            return;
        }
        match self.backend.open(owner, open.panel, open.source) {
            Ok(()) => {
                self.panels.insert(key);
                self.publish(PanelOpened {
                    owner,
                    panel: open.panel,
                });
            }
            Err(reason) => self.failed(owner, open.panel, &reason),
        }
    }

    fn close(&mut self, owner: &str, close: ClosePanel<'_>) {
        let key = (owner.to_string(), close.panel.to_string());
        if !self.panels.contains(&key) {
            self.failed(owner, close.panel, "unknown panel");
            return;
        }
        match self.backend.close(owner, close.panel) {
            Ok(()) => {
                self.panels.remove(&key);
                self.publish(PanelClosed {
                    owner,
                    panel: close.panel,
                });
            }
            Err(reason) => self.failed(owner, close.panel, &reason),
        }
    }

    fn navigate(&mut self, owner: &str, navigate: Navigate<'_>) {
        if !self.owns(owner, navigate.panel) {
            self.failed(owner, navigate.panel, "unknown panel");
            return;
        }
        if let Err(reason) = self.backend.navigate(owner, navigate.panel, navigate.url) {
            self.failed(owner, navigate.panel, &reason);
        }
    }

    fn send_json(&mut self, owner: &str, message: SendJson<'_>) {
        if !self.owns(owner, message.panel) {
            self.failed(owner, message.panel, "unknown panel");
            return;
        }
        if let Err(reason) = self.backend.send_json(owner, message.panel, message.json) {
            self.failed(owner, message.panel, &reason);
        }
    }

    fn owns(&self, owner: &str, panel: &str) -> bool {
        self.panels
            .iter()
            .any(|(candidate_owner, candidate_panel)| {
                candidate_owner == owner && candidate_panel == panel
            })
    }

    fn close_owner(&mut self, owner: &str) {
        let panels: Vec<_> = self
            .panels
            .iter()
            .filter(|(candidate, _)| candidate == owner)
            .cloned()
            .collect();
        for (owner, panel) in panels {
            if let Err(reason) = self.backend.close(&owner, &panel) {
                self.logger.error(
                    ENDPOINT,
                    &format!("closing '{owner}/{panel}' during cleanup: {reason}"),
                );
            }
            self.panels.remove(&(owner, panel));
        }
    }

    fn failed(&self, owner: &str, panel: &str, reason: &str) {
        self.logger
            .error(ENDPOINT, &format!("'{owner}/{panel}': {reason}"));
        self.publish(PanelFailed {
            owner,
            panel,
            reason,
        });
    }

    fn publish<M: EncodeMessage>(&self, message: M) {
        self.bus.publish(Envelope {
            topic: M::TOPIC.to_string(),
            sender: ENDPOINT.to_string(),
            correlation: None,
            payload: message.encode(),
        });
    }

    fn drain_backend_events(&mut self) {
        if let Err(reason) = self.backend.update() {
            self.logger
                .error(ENDPOINT, &format!("updating browser backend: {reason}"));
        }
        for event in self.backend.drain_events() {
            match event {
                BackendEvent::PageMessage { owner, panel, json } if self.owns(&owner, &panel) => {
                    self.publish(PageMessage {
                        owner: &owner,
                        panel: &panel,
                        json: &json,
                    });
                }
                BackendEvent::Closed { owner, panel }
                    if self.panels.remove(&(owner.clone(), panel.clone())) =>
                {
                    self.publish(PanelClosed {
                        owner: &owner,
                        panel: &panel,
                    });
                }
                _ => {}
            }
        }
    }
}

impl Handler for Web {
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic != LifecycleEvent::TOPIC {
            return;
        }
        let Ok(event) = LifecycleEvent::decode(&envelope.payload) else {
            return;
        };
        if matches!(
            event.event,
            Event::Faulted | Event::Reloading | Event::Stopped
        ) {
            self.close_owner(event.extension);
        }
    }
}

impl Module for Web {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe(LifecycleEvent::TOPIC);
        Ok(())
    }

    fn render(&mut self) {
        self.drain_backend_events();
    }

    fn shutdown(&mut self) {
        let owners: Vec<_> = self.panels.iter().map(|(owner, _)| owner.clone()).collect();
        for owner in owners {
            self.close_owner(&owner);
        }
    }

    fn respond(&mut self, sender: &str, payload: &[u8]) -> Option<Vec<u8>> {
        match Command::decode(payload) {
            Ok(Command::Open(open)) => self.open(sender, open),
            Ok(Command::Close(close)) => self.close(sender, close),
            Ok(Command::Navigate(navigate)) => self.navigate(sender, navigate),
            Ok(Command::SendJson(message)) => self.send_json(sender, message),
            Err(err) => self
                .logger
                .error(ENDPOINT, &format!("invalid command from '{sender}': {err}")),
        }
        Some(Vec::new())
    }
}

#[cfg(test)]
mod tests;
