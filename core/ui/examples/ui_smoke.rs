//! Opens a real window, feeds it one fixed `ui/spec` (mimicking the "notes"
//! example), and renders it for a few seconds with mouse/keyboard routed
//! through egui. Run with: cargo run -p ui --example ui_smoke

use bones_messages::ui::{Spec, Widget};
use bones_messages::{EncodeMessage, Message};
use bus::{Bus, Envelope, Handler, Module, ModuleContext, ServiceRegistry};
use logging::Logger;
use platform::Platform;
use renderer::Renderer;
use ui::Ui;

fn main() -> Result<(), String> {
    let mut platform = Platform::new("ui smoke test", 480, 320)?;
    let window = platform.take_window().expect("window should be available");

    let bus = Bus::new();
    let mut renderer = Renderer::new(bus.clone(), Logger::default());
    let mut services = ServiceRegistry::new();
    services.provide(window)?;
    let mut ctx = ModuleContext::new(&mut services);
    renderer.init(&mut ctx)?;

    let mut ui = Ui::new(bus.clone(), Logger::default());

    let spec = Spec {
        title: "notes",
        widgets: vec![
            Widget::TextEdit {
                id: 1,
                text: "buy milk",
            },
            Widget::Button {
                id: 2,
                label: "Add",
            },
            Widget::Label {
                text: "existing note",
            },
        ],
    };

    for frame in 0..240 {
        ui.handle(&Envelope {
            topic: Spec::TOPIC.to_string(),
            sender: "notes".to_string(),
            correlation: None,
            payload: spec.encode(),
        });

        platform.poll_events_with(&bus, "platform", |event| ui.feed_event(event));
        if platform.quit_requested() {
            break;
        }

        renderer.handle(&Envelope {
            topic: "gfx/clear".to_string(),
            sender: "smoke".to_string(),
            correlation: None,
            payload: vec![30, 30, 60, 255],
        });
        ui.update(&mut renderer, 480, 320);
        renderer.present();

        if frame % 60 == 0 {
            println!("frame {frame}");
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    println!("rendered without error");
    Ok(())
}
