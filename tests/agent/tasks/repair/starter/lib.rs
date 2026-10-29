//! A square with a trail, moved by the arrow keys.

use std::cell::{Cell, RefCell};

use bones_wasm_sdk::bindings::bones::extension::host_api::{log, subscribe, Level};
use bones_wasm_sdk::messages::gfx::{Clear, DrawRect};
use bones_wasm_sdk::messages::tick::Tick;
use bones_wasm_sdk::{bus, Guest, Inbound};

const SPEED: f32 = 240.0;
const SIZE: u32 = 40;
const TRAIL: usize = 100;

thread_local! {
    static POSITION: Cell<(f32, f32)> = const { Cell::new((380.0, 280.0)) };
    static HELD: Cell<(f32, f32)> = const { Cell::new((0.0, 0.0)) };
    static TRAIL_POINTS: RefCell<Vec<(i32, i32)>> = const { RefCell::new(Vec::new()) };
}

struct Component;

impl Guest for Component {
    fn init() {
        bus::subscribe::<Tick>();
        subscribe("input/key-dwn");
        subscribe("input/key-up");
        log(Level::Info, "ready");
    }

    fn shutdown() {}

    fn on_tick(dt: f32) {
        let (x, y) = POSITION.get();
        let (dx, dy) = HELD.get();
        let (x, y) = (x + dx * SPEED * dt, y + dy * SPEED * dt);
        POSITION.set((x, y));

        bus::publish(&Clear { r: 10, g: 10, b: 20, a: 255 });
        TRAIL_POINTS.with(|trail| {
            let mut trail = trail.borrow_mut();
            trail.push((x as i32 + 15, y as i32 + 15));
            // Every point ever visited, drawn as a fading dot.
            for (i, (px, py)) in trail.iter().enumerate() {
                for _ in 0..20 {
                    bus::publish(&DrawRect {
                        x: *px,
                        y: *py,
                        w: 10,
                        h: 10,
                        filled: true,
                        color: (80, 200, 255, (i % 255) as u8),
                        layer: 0,
                        screen_space: false,
                    });
                }
            }
        });
        bus::publish(&DrawRect {
            x: x as i32,
            y: y as i32,
            w: SIZE,
            h: SIZE,
            filled: true,
            color: (80, 200, 255, 255),
            layer: 1,
            screen_space: false,
        });
        let _ = TRAIL;
    }

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        match Inbound::decode(&topic, &payload) {
            Ok(Inbound::KeyDown(key)) => hold(key.key, 1.0),
            Ok(Inbound::KeyUp(key)) => hold(key.key, 0.0),
            _ => {}
        }
        None
    }
}

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
