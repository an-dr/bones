//! Opens a real window, takes it from Platform, clears it a few times, and
//! presents. Run with: cargo run -p renderer --example render_smoke

use bus::{Bus, Envelope, Handler};
use logging::Logger;
use platform::Platform;
use renderer::Renderer;

fn main() -> Result<(), String> {
    let mut platform = Platform::new("renderer smoke test", 320, 240)?;
    let window = platform.take_window().expect("window should be available");
    let mut renderer = Renderer::new(window, Logger::default());

    let bus = Bus::new();
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
