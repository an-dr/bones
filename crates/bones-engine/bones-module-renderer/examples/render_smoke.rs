//! Opens a real window, takes it from Platform, clears it a few times, and
//! presents. Run with: cargo run -p renderer --example render_smoke

use bones_kernel::bus::{Bus, Envelope, Handler, Module, ModuleContext, ServiceRegistry};
use bones_kernel::logging::Logger;
use bones_kernel::platform::Platform;
use bones_module_renderer::Renderer;

fn main() -> Result<(), String> {
    let mut platform = Platform::new("renderer smoke test", 320, 240)?;
    let window = platform.take_window().expect("window should be available");

    let bus = Bus::new();
    let mut renderer = Renderer::new(bus.clone(), Logger::default());
    let mut services = ServiceRegistry::new();
    services.provide(window)?;
    let mut ctx = ModuleContext::new(&mut services);
    renderer.init(&mut ctx)?;

    for _ in 0..60 {
        renderer.handle(&Envelope {
            topic: "gfx/clear".to_string(),
            sender: "smoke".to_string(),
            correlation: None,
            payload: vec![30, 30, 60, 255],
        });
        renderer.present();
        platform.poll_events(&bus, "platform");
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    println!("cleared and presented without error");
    Ok(())
}
