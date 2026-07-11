//! Opens a real window and polls it for ~2s without crashing. Run with:
//! cargo run -p platform --example smoke

use bus::Bus;
use platform::Platform;

fn main() -> Result<(), String> {
    let mut platform = Platform::new("bones smoke test", 320, 240)?;
    let bus = Bus::new();
    for _ in 0..120 {
        platform.poll_events(&bus, "platform");
        bus.dispatch();
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    println!("window created and polled without error");
    Ok(())
}
