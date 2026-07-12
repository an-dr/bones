wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, publish, subscribe, Level};

const SPRITE_PNG: &[u8] = include_bytes!("assets/robot_william.png");
const SPRITE_ID: u32 = 1;
// robot_william.png is a 256x64 strip of four 64x64 frames; this rung only
// ever draws the first one.
const FRAME_SIZE: i32 = 64;
const DST_X: i32 = 368;
const DST_Y: i32 = 268;

fn load_sprite_payload() -> Vec<u8> {
    let mut payload = SPRITE_ID.to_le_bytes().to_vec();
    payload.extend_from_slice(SPRITE_PNG);
    payload
}

fn draw_sprite_payload() -> Vec<u8> {
    let mut payload = Vec::with_capacity(28);
    payload.extend_from_slice(&SPRITE_ID.to_le_bytes());
    payload.extend_from_slice(&DST_X.to_le_bytes());
    payload.extend_from_slice(&DST_Y.to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes()); // src_x
    payload.extend_from_slice(&0i32.to_le_bytes()); // src_y
    payload.extend_from_slice(&(FRAME_SIZE as u32).to_le_bytes()); // src_w
    payload.extend_from_slice(&(FRAME_SIZE as u32).to_le_bytes()); // src_h
    payload
}

struct Component;

impl Guest for Component {
    fn init() {
        subscribe("core/tick");
        publish("gfx/load-sprite", &load_sprite_payload());
        log(Level::Info, "sprite_demo extension: init, sprite loaded");
    }

    fn on_tick(_dt: f32) {
        publish("gfx/clear", &[20, 20, 30, 255]);
        publish("gfx/draw-sprite", &draw_sprite_payload());
    }

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) {}
}

export!(Component);
