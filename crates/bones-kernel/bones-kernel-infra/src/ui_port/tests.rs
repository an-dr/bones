use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bones_messages::ui::kinds::Label;
use bones_messages::ui::{Event, Root, Spec, WidgetProps};

use super::{Frame, UiPort, UiService, UiView, ViewModule};
use crate::bus::{Module, ModuleContext, ServiceRegistry};

#[derive(Default)]
struct FakePort {
    shown: Vec<(String, Spec)>,
    images: Vec<(String, u32)>,
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

    fn set_image(&mut self, owner: &str, handle: u32, _w: u32, _h: u32, _rgba: Vec<u8>) {
        self.images.push((owner.to_string(), handle));
    }

    fn free_image(&mut self, _owner: &str, _handle: u32) {}

    fn set_theme(&mut self, _theme: super::Theme) {}
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

    fn view(&mut self, frame: &mut Frame<'_>) -> Spec {
        if self.clicks > 0 {
            frame.window(
                "dialog",
                Spec {
                    root: Root::Modal {
                        title: "Clicked".into(),
                    },
                    body: Label { text: "!".into() }.node(0),
                },
            );
        }
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
    // The view shows a dialog once it has a click, under its own name.
    assert_eq!(
        shown,
        [
            ("app/dialog", "!".to_string()),
            ("app", "1".to_string()),
            ("app/dialog", "!".to_string()),
            ("app", "1".to_string())
        ]
    );
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

#[test]
fn events_from_an_extra_tree_and_images_go_through_the_frame() {
    struct Uploader {
        seen: Vec<Event>,
    }
    impl UiView for Uploader {
        fn on_event(&mut self, event: Event) {
            self.seen.push(event);
        }
        fn view(&mut self, frame: &mut Frame<'_>) -> Spec {
            frame.set_image(7, 1, 1, vec![0; 4]);
            frame.window(
                "dialog",
                Spec {
                    root: Root::Screen,
                    body: Label { text: "d".into() }.node(0),
                },
            );
            Spec {
                root: Root::Screen,
                body: Label { text: "m".into() }.node(0),
            }
        }
    }
    let port = Arc::new(Mutex::new(FakePort::default()));
    let service: UiService = port.clone();
    let mut services = ServiceRegistry::new();
    services.provide(service).unwrap();
    let mut module = ViewModule::new("app", Uploader { seen: Vec::new() });
    module.init(&mut ModuleContext::new(&mut services)).unwrap();

    module.after_tick();
    port.lock()
        .unwrap()
        .queued
        .insert("app/dialog".into(), vec![Event::Clicked { id: 4 }]);
    module.after_tick();

    assert_eq!(module.view.seen, [Event::Clicked { id: 4 }]);
    assert_eq!(port.lock().unwrap().images[0], ("app".to_string(), 7));
}
