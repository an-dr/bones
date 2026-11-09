use std::sync::{Arc, Mutex};

use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext, ServiceRegistry};
use bones_kernel_infra::draw_target::{DrawTarget, DrawTargetService, UiMesh};
use bones_kernel_infra::logging::{Level, Logger, RecordingSink};
use bones_kernel_infra::ui_port::{Theme, UiService};
use bones_messages::ui::kinds::{Button, Label};
use bones_messages::ui::{Layout, Node, Root, Spec, WidgetProps};
use bones_messages::{EncodeMessage, Message};

use super::Ui;
use crate::draw_context::DrawContext;
use crate::kind::Kind;
use bones_messages::ui::kinds::{Fit, Image};
use bones_messages::ui::{Event, SetImage};

/// Everything the ui module asks of a surface, recorded instead of drawn.
///
/// The point of the `draw-target` service is that this is a complete
/// substitute for the renderer as far as ui is concerned — no window, no
/// SDL, no `bones-module-renderer` in this crate's dependency graph at all.
#[derive(Default)]
struct RecordingTarget {
    textures_set: Vec<u64>,
    texture_regions: Vec<u64>,
    textures_freed: Vec<u64>,
    meshes: Vec<UiMesh>,
}

#[derive(Clone, Default)]
struct SharedTarget(Arc<Mutex<RecordingTarget>>);

impl DrawTarget for SharedTarget {
    fn size(&self) -> (u32, u32) {
        (480, 320)
    }

    fn set_ui_texture(
        &mut self,
        id: u64,
        _width: u32,
        _height: u32,
        _rgba: &[u8],
    ) -> Result<(), String> {
        self.0.lock().unwrap().textures_set.push(id);
        Ok(())
    }

    fn update_ui_texture_region(
        &mut self,
        id: u64,
        _x: u32,
        _y: u32,
        _width: u32,
        _height: u32,
        _rgba: &[u8],
    ) -> Result<(), String> {
        self.0.lock().unwrap().texture_regions.push(id);
        Ok(())
    }

    fn free_ui_texture(&mut self, id: u64) {
        self.0.lock().unwrap().textures_freed.push(id);
    }

    fn draw_ui_mesh(&mut self, mesh: &UiMesh) -> Result<(), String> {
        self.0.lock().unwrap().meshes.push(mesh.clone());
        Ok(())
    }
}

fn ui_with_target(target: SharedTarget) -> Ui {
    let mut ui = Ui::new(Bus::new(), Logger::default());
    let mut services = ServiceRegistry::new();
    services
        .provide::<DrawTargetService>(Box::new(target))
        .unwrap();
    let mut ctx = ModuleContext::new(&mut services);
    ui.init(&mut ctx).unwrap();
    ui
}

fn spec_envelope() -> Envelope {
    let spec = Spec {
        root: Root::Window {
            title: "notes".into(),
        },
        body: Node::container(
            Layout::Vertical,
            vec![
                Label {
                    text: "existing note".into(),
                }
                .node(0),
                Button {
                    label: "Add".into(),
                }
                .node(2),
            ],
        ),
    };
    Envelope {
        topic: Spec::TOPIC.to_string(),
        sender: "notes".to_string(),
        correlation: None,
        payload: spec.encode(),
    }
}

#[test]
fn a_spec_is_drawn_through_whatever_surface_provides_draw_target() {
    let target = SharedTarget::default();
    let mut ui = ui_with_target(target.clone());

    // Two frames, and the spec is republished for each: ADR-005 is
    // immediate-mode, so `update` drains what it drew. egui's first frame
    // over a fresh context lays an auto-sized window out without painting
    // it, which is a property of egui rather than of this wiring — the
    // real loop republishes every tick, so steady state is what matters.
    for _ in 0..2 {
        ui.handle(&spec_envelope());
        ui.update();
    }

    let recorded = target.0.lock().unwrap();
    assert!(
        !recorded.meshes.is_empty(),
        "a window with a label and a button should tessellate to at least one mesh"
    );
    assert!(
        !recorded.textures_set.is_empty(),
        "egui's font atlas should upload through set_ui_texture"
    );
    assert!(
        recorded
            .meshes
            .iter()
            .all(|mesh| !mesh.vertices.is_empty() && !mesh.indices.is_empty()),
        "no empty mesh should be submitted"
    );
}

#[test]
fn init_without_a_draw_target_is_an_error_not_a_silently_blind_module() {
    let mut ui = Ui::new(Bus::new(), Logger::default());
    let mut services = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut services);

    let result = ui.init(&mut ctx);

    assert!(result.is_err(), "ui has nothing to draw on without one");
}

#[test]
fn update_before_init_draws_nothing_instead_of_panicking() {
    let mut ui = Ui::new(Bus::new(), Logger::default());

    ui.handle(&spec_envelope());
    ui.update();
}

#[test]
fn the_module_name_is_the_bus_endpoint_ui_specs_are_addressed_to() {
    let ui = ui_with_target(SharedTarget::default());

    assert_eq!(ui.name(), "ui");
}

#[test]
fn a_broken_widget_republished_every_frame_is_reported_once() {
    let sink = RecordingSink::new();
    let mut ui = Ui::new(Bus::new(), Logger::new(Arc::new(sink.clone())));
    let mut services = ServiceRegistry::new();
    services
        .provide::<DrawTargetService>(Box::new(SharedTarget::default()))
        .unwrap();
    ui.init(&mut ModuleContext::new(&mut services)).unwrap();
    let spec = Spec {
        root: Root::Screen,
        body: Node::Widget {
            kind: "nobody/registered-this".into(),
            id: 1,
            props: vec![],
        },
    };

    for _ in 0..3 {
        ui.handle(&Envelope {
            topic: Spec::TOPIC.to_string(),
            sender: "game".to_string(),
            correlation: None,
            payload: spec.encode(),
        });
        ui.update();
    }

    let warnings: Vec<String> = sink
        .records()
        .into_iter()
        .filter(|(level, _, _)| *level == Level::Warn)
        .map(|(_, _, message)| message)
        .collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("'game'") && warnings[0].contains("nobody/registered-this"));
}

/// Reports a click every frame it is drawn, so a test needs no pointer.
struct AlwaysClicked;

impl Kind for AlwaysClicked {
    type Props = Label;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Label, cx: &mut DrawContext<'_>) {
        ui.label(props.text);
        cx.emit(Event::Clicked { id });
    }
}

#[test]
fn a_native_tree_draws_through_the_ui_service_and_gets_its_events_back() {
    let target = SharedTarget::default();
    let mut ui = Ui::new(Bus::new(), Logger::default());
    ui.register(AlwaysClicked);
    let mut services = ServiceRegistry::new();
    services
        .provide::<DrawTargetService>(Box::new(target.clone()))
        .unwrap();
    ui.init(&mut ModuleContext::new(&mut services)).unwrap();
    let port: UiService = services.get::<UiService>().unwrap().clone();
    let spec = Spec {
        root: Root::Screen,
        body: Label { text: "go".into() }.node(7),
    };

    for _ in 0..2 {
        port.lock().unwrap().show("app", spec.clone());
        ui.update();
    }

    let events = port.lock().unwrap().take_events("app");
    assert_eq!(events, [Event::Clicked { id: 7 }, Event::Clicked { id: 7 }]);
    assert!(port.lock().unwrap().take_events("app").is_empty());
    assert!(!target.0.lock().unwrap().meshes.is_empty());
}

fn screen_of(text: &str, id: u32) -> Spec {
    Spec {
        root: Root::Screen,
        body: Label { text: text.into() }.node(id),
    }
}

#[test]
fn one_owner_holds_the_screen_and_a_second_claim_is_refused_once() {
    let sink = RecordingSink::new();
    let mut ui = Ui::new(Bus::new(), Logger::new(Arc::new(sink.clone())));
    ui.register(AlwaysClicked);
    let mut services = ServiceRegistry::new();
    services
        .provide::<DrawTargetService>(Box::new(SharedTarget::default()))
        .unwrap();
    ui.init(&mut ModuleContext::new(&mut services)).unwrap();
    let port: UiService = services.get::<UiService>().unwrap().clone();

    for _ in 0..3 {
        port.lock().unwrap().show("first", screen_of("a", 1));
        port.lock().unwrap().show("second", screen_of("b", 2));
        ui.update();
    }

    let mut port = port.lock().unwrap();
    assert_eq!(port.take_events("first").len(), 3);
    assert!(port.take_events("second").is_empty());
    let warnings: Vec<String> = sink
        .records()
        .into_iter()
        .filter(|(level, _, _)| *level == Level::Warn)
        .map(|(_, _, message)| message)
        .collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("'second'") && warnings[0].contains("'first'"));
}

#[test]
fn zoom_scales_what_is_drawn() {
    let tallest = |zoom: f32| {
        let target = SharedTarget::default();
        let mut ui = Ui::new(Bus::new(), Logger::default());
        let mut services = ServiceRegistry::new();
        services
            .provide::<DrawTargetService>(Box::new(target.clone()))
            .unwrap();
        ui.init(&mut ModuleContext::new(&mut services)).unwrap();
        let port: UiService = services.get::<UiService>().unwrap().clone();
        port.lock().unwrap().set_zoom(zoom);
        // A floating window sizes itself to its content, unlike a Screen,
        // whose central panel always fills the surface.
        let window = Spec {
            root: Root::Window { title: "w".into() },
            body: Label {
                text: "Hello".into(),
            }
            .node(0),
        };
        for _ in 0..2 {
            port.lock().unwrap().show("app", window.clone());
            ui.update();
        }
        let recorded = target.0.lock().unwrap();
        recorded
            .meshes
            .iter()
            .flat_map(|mesh| mesh.vertices.iter())
            .map(|vertex| vertex.y)
            .fold(0.0_f32, f32::max)
    };

    let (normal, doubled) = (tallest(1.0), tallest(2.0));

    assert!(
        doubled > normal * 1.5,
        "zoom 2 should draw about twice as large: {normal} vs {doubled}"
    );
}

fn image_screen(handle: u32) -> Spec {
    Spec {
        root: Root::Screen,
        body: Image {
            handle,
            width: 32.0,
            height: 32.0,
            fit: Fit::Cover,
            corner: 0.0,
        }
        .node(0),
    }
}

fn warnings(sink: &RecordingSink) -> Vec<String> {
    sink.records()
        .into_iter()
        .filter(|(level, _, _)| *level == Level::Warn)
        .map(|(_, _, message)| message)
        .collect()
}

#[test]
fn images_upload_once_through_either_door_and_draw_by_handle() {
    let sink = RecordingSink::new();
    let target = SharedTarget::default();
    let mut ui = Ui::new(Bus::new(), Logger::new(Arc::new(sink.clone())));
    let mut services = ServiceRegistry::new();
    services
        .provide::<DrawTargetService>(Box::new(target.clone()))
        .unwrap();
    ui.init(&mut ModuleContext::new(&mut services)).unwrap();
    let port: UiService = services.get::<UiService>().unwrap().clone();

    port.lock().unwrap().set_image("app", 1, 2, 1, vec![255; 8]);
    ui.handle(&Envelope {
        topic: SetImage::TOPIC.to_string(),
        sender: "ext".to_string(),
        correlation: None,
        payload: SetImage {
            handle: 1,
            width: 1,
            height: 1,
            rgba: &[0, 0, 255, 255],
        }
        .encode(),
    });
    for _ in 0..2 {
        port.lock().unwrap().show("app", image_screen(1));
        ui.update();
    }

    // The font atlas plus one texture per owner's image, each uploaded once.
    assert_eq!(target.0.lock().unwrap().textures_set.len(), 3);
    assert!(warnings(&sink).is_empty(), "{:?}", warnings(&sink));
}

#[test]
fn an_image_whose_size_and_pixels_disagree_is_refused_with_a_warning() {
    let sink = RecordingSink::new();
    let mut ui = Ui::new(Bus::new(), Logger::new(Arc::new(sink.clone())));
    let mut services = ServiceRegistry::new();
    services
        .provide::<DrawTargetService>(Box::new(SharedTarget::default()))
        .unwrap();
    ui.init(&mut ModuleContext::new(&mut services)).unwrap();
    let port: UiService = services.get::<UiService>().unwrap().clone();

    port.lock().unwrap().set_image("app", 4, 10, 10, vec![0; 3]);
    ui.update();

    let warnings = warnings(&sink);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("image 4"));
}

#[test]
fn the_owners_theme_choice_overrides_the_desktop_mode() {
    let mut ui = Ui::new(Bus::new(), Logger::default());
    ui.use_native_look();
    let mut services = ServiceRegistry::new();
    services
        .provide::<DrawTargetService>(Box::new(SharedTarget::default()))
        .unwrap();
    ui.init(&mut ModuleContext::new(&mut services)).unwrap();
    let port: UiService = services.get::<UiService>().unwrap().clone();

    for (theme, dark) in [
        (Theme::Dark, true),
        (Theme::Light, false),
        (Theme::Dark, true),
    ] {
        port.lock().unwrap().set_theme(theme);
        ui.update();
        assert_eq!(ui.ctx.style().visuals.dark_mode, dark, "{theme:?}");
    }
}
