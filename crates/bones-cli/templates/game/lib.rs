//! A square the arrow keys move, drawn every frame.

use std::cell::Cell;

use bones_wasm_sdk::bindings::bones::extension::host_api::{log, Level};
use bones_wasm_sdk::messages::gfx::{Clear, DrawRect};
use bones_wasm_sdk::messages::input::{KeyDown, KeyUp};
use bones_wasm_sdk::messages::tick::Tick;
use bones_wasm_sdk::{bus, Guest, Inbound};

/// Pixels per second.
const SPEED: f32 = 240.0;
const SIZE: u32 = 40;

thread_local! {
    static POSITION: Cell<(f32, f32)> = const { Cell::new((380.0, 280.0)) };
    static HELD: Cell<(f32, f32)> = const { Cell::new((0.0, 0.0)) };
}

struct Component;

impl Guest for Component {
    fn init() {
        bus::subscribe::<Tick>();
        bus::subscribe::<KeyDown>();
        bus::subscribe::<KeyUp>();
        log(Level::Info, "ready: move the square with the arrow keys");
    }

    fn shutdown() {}

    fn on_tick(dt: f32) {
        let (x, y) = POSITION.get();
        let (dx, dy) = HELD.get();
        let (x, y) = (x + dx * SPEED * dt, y + dy * SPEED * dt);
        POSITION.set((x, y));

        bus::publish(&Clear { r: 20, g: 20, b: 30, a: 255 });
        bus::publish(&DrawRect {
            x: x as i32,
            y: y as i32,
            w: SIZE,
            h: SIZE,
            filled: true,
            color: (80, 200, 255, 255),
            layer: 0,
            screen_space: false,
        });
    }

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        match Inbound::decode(&topic, &payload) {
            Ok(Inbound::KeyDown(key)) => hold(key.key, 1.0),
            Ok(Inbound::KeyUp(key)) => hold(key.key, 0.0),
            Ok(_) => {}
            Err(error) => log(Level::Warn, &format!("bad {topic}: {error}")),
        }
        None
    }
}

/// Sets the held direction for an arrow key; `amount` is 1 down, 0 up.
fn hold(key: &str, amount: f32) {
    let (dx, dy) = HELD.get();
    let held = match key {
        "Left" => (-amount, dy),
        "Right" => (amount, dy),
        "Up" => (dx, -amount),
        "Down" => (dx, amount),
        _ => return,
    };
    HELD.set(held);
}

bones_wasm_sdk::export!(Component);
