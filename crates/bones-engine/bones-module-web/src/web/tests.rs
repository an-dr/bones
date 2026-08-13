use std::sync::{Arc, Mutex};

use bones_messages::lifecycle::{Event, LifecycleEvent};
use bones_messages::web::{
    ClosePanel, Command, Navigate, OpenPanel, PageMessage, PanelClosed, PanelFailed, PanelOpened,
    PanelSource, SendJson,
};
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use bones_kernel::bus::{Envelope, Handler, Module, ModuleContext, ServiceRegistry};
use bones_kernel::logging::{Logger, RecordingSink};

use super::*;

#[derive(Clone, Default)]
struct FakeState {
    calls: Arc<Mutex<Vec<String>>>,
    events: Arc<Mutex<Vec<BackendEvent>>>,
    updates: Arc<Mutex<usize>>,
}

struct FakeBackend {
    state: FakeState,
}

impl Backend for FakeBackend {
    fn open(&mut self, owner: &str, panel: &str, source: PanelSource<'_>) -> Result<(), String> {
        let source = match source {
            PanelSource::Html(_) => "html",
            PanelSource::Url(_) => "url",
        };
        self.state
            .calls
            .lock()
            .unwrap()
            .push(format!("open:{owner}:{panel}:{source}"));
        Ok(())
    }

    fn close(&mut self, owner: &str, panel: &str) -> Result<(), String> {
        self.state
            .calls
            .lock()
            .unwrap()
            .push(format!("close:{owner}:{panel}"));
        Ok(())
    }

    fn navigate(&mut self, owner: &str, panel: &str, url: &str) -> Result<(), String> {
        self.state
            .calls
            .lock()
            .unwrap()
            .push(format!("navigate:{owner}:{panel}:{url}"));
        Ok(())
    }

    fn send_json(&mut self, owner: &str, panel: &str, json: &str) -> Result<(), String> {
        self.state
            .calls
            .lock()
            .unwrap()
            .push(format!("json:{owner}:{panel}:{json}"));
        Ok(())
    }

    fn update(&mut self) -> Result<(), String> {
        *self.state.updates.lock().unwrap() += 1;
        Ok(())
    }

    fn drain_events(&mut self) -> Vec<BackendEvent> {
        std::mem::take(&mut *self.state.events.lock().unwrap())
    }
}

fn setup() -> (Web, FakeState, Bus, Arc<Mutex<Vec<Envelope>>>) {
    let bus = Bus::new();
    let state = FakeState::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let endpoint = bus.register("watcher", move |event: &Envelope| {
        sink.lock().unwrap().push(event.clone());
    });
    endpoint.subscribe("web/*");
    let web = Web::new(
        bus.clone(),
        Logger::default(),
        FakeBackend {
            state: state.clone(),
        },
    );
    (web, state, bus, events)
}

fn command(web: &mut Web, sender: &str, command: Command<'_>) {
    assert_eq!(web.respond(sender, &command.encode()), Some(Vec::new()));
}

#[test]
fn module_identity_and_lifecycle_subscription_are_explicit() {
    let (mut web, _, _, _) = setup();
    let mut services = ServiceRegistry::new();
    let mut context = ModuleContext::new(&mut services);

    assert_eq!(web.name(), "web");
    web.init(&mut context).unwrap();
    assert_eq!(
        context.into_subscriptions(),
        vec![LifecycleEvent::TOPIC.to_string()]
    );
}

#[test]
fn render_updates_the_native_backend() {
    let (mut web, state, _, _) = setup();
    web.render();

    assert_eq!(*state.updates.lock().unwrap(), 1);
}

#[test]
fn panel_ids_are_scoped_by_the_host_stamped_sender() {
    let (mut web, state, bus, events) = setup();
    for owner in ["alice", "bob"] {
        command(
            &mut web,
            owner,
            Command::Open(OpenPanel {
                panel: "status",
                source: PanelSource::Html("<p>ok</p>"),
            }),
        );
    }
    command(
        &mut web,
        "alice",
        Command::Open(OpenPanel {
            panel: "status",
            source: PanelSource::Url("https://duplicate.invalid"),
        }),
    );
    bus.dispatch();

    assert_eq!(
        *state.calls.lock().unwrap(),
        vec!["open:alice:status:html", "open:bob:status:html"]
    );
    let events = events.lock().unwrap();
    let opened: Vec<_> = events
        .iter()
        .filter(|event| event.topic == PanelOpened::TOPIC)
        .map(|event| {
            PanelOpened::decode(&event.payload)
                .unwrap()
                .owner
                .to_string()
        })
        .collect();
    assert_eq!(opened, vec!["alice", "bob"]);
    let failed = events
        .iter()
        .find(|event| event.topic == PanelFailed::TOPIC)
        .map(|event| PanelFailed::decode(&event.payload).unwrap())
        .unwrap();
    assert_eq!((failed.owner, failed.panel), ("alice", "status"));
}

#[test]
fn one_owner_cannot_operate_another_owners_panel() {
    let (mut web, state, bus, events) = setup();
    command(
        &mut web,
        "alice",
        Command::Open(OpenPanel {
            panel: "private",
            source: PanelSource::Html("secret"),
        }),
    );
    command(
        &mut web,
        "bob",
        Command::Navigate(Navigate {
            panel: "private",
            url: "https://attacker.invalid",
        }),
    );
    command(
        &mut web,
        "bob",
        Command::SendJson(SendJson {
            panel: "private",
            json: "{}",
        }),
    );
    command(
        &mut web,
        "bob",
        Command::Close(ClosePanel { panel: "private" }),
    );
    command(
        &mut web,
        "alice",
        Command::Close(ClosePanel { panel: "private" }),
    );
    bus.dispatch();

    assert_eq!(
        *state.calls.lock().unwrap(),
        vec!["open:alice:private:html", "close:alice:private"]
    );
    assert_eq!(
        events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.topic == PanelFailed::TOPIC)
            .count(),
        3
    );
}

#[test]
fn owner_lifecycle_and_module_shutdown_close_owned_panels() {
    let (mut web, state, _, _) = setup();
    for (owner, panel) in [("alice", "a"), ("alice", "b"), ("bob", "a")] {
        command(
            &mut web,
            owner,
            Command::Open(OpenPanel {
                panel,
                source: PanelSource::Html(""),
            }),
        );
    }
    web.handle(&Envelope {
        topic: LifecycleEvent::TOPIC.to_string(),
        sender: "engine".to_string(),
        correlation: None,
        payload: LifecycleEvent {
            event: Event::Faulted,
            extension: "alice",
        }
        .encode(),
    });
    web.shutdown();

    let calls = state.calls.lock().unwrap();
    assert!(calls.contains(&"close:alice:a".to_string()));
    assert!(calls.contains(&"close:alice:b".to_string()));
    assert!(calls.contains(&"close:bob:a".to_string()));
}

#[test]
fn backend_page_and_close_events_route_only_for_live_panels() {
    let (mut web, state, bus, events) = setup();
    command(
        &mut web,
        "dashboard",
        Command::Open(OpenPanel {
            panel: "status",
            source: PanelSource::Html(""),
        }),
    );
    state.events.lock().unwrap().extend([
        BackendEvent::PageMessage {
            owner: "dashboard".to_string(),
            panel: "status".to_string(),
            json: r#"{"ready":true}"#.to_string(),
        },
        BackendEvent::Closed {
            owner: "dashboard".to_string(),
            panel: "status".to_string(),
        },
        BackendEvent::PageMessage {
            owner: "dashboard".to_string(),
            panel: "status".to_string(),
            json: r#"{"stale":true}"#.to_string(),
        },
    ]);
    web.render();
    bus.dispatch();

    let events = events.lock().unwrap();
    let page_messages: Vec<_> = events
        .iter()
        .filter(|event| event.topic == PageMessage::TOPIC)
        .map(|event| {
            PageMessage::decode(&event.payload)
                .unwrap()
                .json
                .to_string()
        })
        .collect();
    assert_eq!(page_messages, vec![r#"{"ready":true}"#]);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.topic == PanelClosed::TOPIC)
            .count(),
        1
    );
}

#[test]
fn malformed_direct_commands_are_logged_without_touching_the_backend() {
    let bus = Bus::new();
    let state = FakeState::default();
    let sink = RecordingSink::new();
    let mut web = Web::new(
        bus,
        Logger::new(Arc::new(sink.clone())),
        FakeBackend {
            state: state.clone(),
        },
    );

    assert_eq!(web.respond("guest", &[255]), Some(Vec::new()));

    assert!(state.calls.lock().unwrap().is_empty());
    assert!(sink
        .records()
        .iter()
        .any(|(_, category, message)| category == "web"
            && message.contains("invalid command from 'guest'")));
}
