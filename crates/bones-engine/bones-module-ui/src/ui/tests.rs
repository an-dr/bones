use std::sync::{Arc, Mutex};

use bones_kernel::bus::{Bus, Envelope, Handler, Module, ModuleContext, ServiceRegistry};
use bones_kernel::draw_target::{DrawTarget, DrawTargetService, UiMesh};
use bones_kernel::logging::Logger;
use bones_messages::ui::{Spec, Widget};
use bones_messages::{EncodeMessage, Message};

use super::Ui;

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
        title: "notes",
        widgets: vec![
            Widget::Label {
                text: "existing note",
            },
            Widget::Button {
                id: 2,
                label: "Add",
            },
        ],
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
