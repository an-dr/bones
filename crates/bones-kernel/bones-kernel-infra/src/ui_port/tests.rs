use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bones_messages::ui::kinds::Label;
use bones_messages::ui::{Event, Root, Spec, WidgetProps};

use super::{UiPort, UiService, UiView, ViewModule};
use crate::bus::{Module, ModuleContext, ServiceRegistry};

#[derive(Default)]
struct FakePort {
    shown: Vec<(String, Spec)>,
    queued: HashMap<String, Vec<Event>>,
}

impl UiPort for FakePort {
    fn show(&mut self, owner: &str, spec: Spec) {
        self.shown.push((owner.to_string(), spec));
    }

    fn take_events(&mut self, owner: &str) -> Vec<Event> {
        self.queued.remove(owner).unwrap_or_default()
    }

    fn set_zoom(&mut self, _zoom: f32) {}
}

/// Counts clicks and shows the count.
#[derive(Default)]
struct Counter {
    clicks: u32,
}

impl UiView for Counter {
    fn on_event(&mut self, event: Event) {
        if let Event::Clicked { .. } = event {
            self.clicks += 1;
        }
    }

    fn view(&mut self) -> Spec {
        Spec {
            root: Root::Screen,
            body: Label {
                text: self.clicks.to_string(),
            }
            .node(0),
        }
    }
}

fn label_of(spec: &Spec) -> String {
    spec.body.widgets_of::<Label>()[0].1.text.clone()
}

#[test]
fn a_view_gets_its_events_then_shows_the_tree_they_produced() {
    let port = Arc::new(Mutex::new(FakePort::default()));
    let service: UiService = port.clone();
    let mut services = ServiceRegistry::new();
    services.provide(service).unwrap();
    let mut module = ViewModule::new("app", Counter::default());
    module.init(&mut ModuleContext::new(&mut services)).unwrap();

    port.lock()
        .unwrap()
        .queued
        .insert("app".into(), vec![Event::Clicked { id: 1 }]);
    module.after_tick();
    module.after_tick();

    let port = port.lock().unwrap();
    let shown: Vec<(&str, String)> = port
        .shown
        .iter()
        .map(|(owner, spec)| (owner.as_str(), label_of(spec)))
        .collect();
    assert_eq!(shown, [("app", "1".to_string()), ("app", "1".to_string())]);
}

#[test]
fn without_the_ui_service_init_says_what_to_compose() {
    let mut services = ServiceRegistry::new();
    let mut module = ViewModule::new("app", Counter::default());

    let error = module
        .init(&mut ModuleContext::new(&mut services))
        .unwrap_err();

    assert!(error.contains(".ui()"), "{error}");
}
