wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, publish, subscribe, Level};
use bones_messages::gfx::{Clear, DrawSprite, LoadSprite};
use bones_messages::{EncodeMessage, Message};

const SPRITE_PNG: &[u8] = include_bytes!("assets/robot_william.png");
const SPRITE_ID: u32 = 1;
// robot_william.png is a 256x64 strip of four 64x64 frames; this rung only
// ever draws the first one.
const FRAME_SIZE: u32 = 64;
const DST_X: i32 = 368;
const DST_Y: i32 = 268;

struct Component;

impl Guest for Component {
    fn init() {
        subscribe("core/tick");
        let load = LoadSprite {
            id: SPRITE_ID,
            png_bytes: SPRITE_PNG,
        };
        publish(LoadSprite::TOPIC, &load.encode());
        log(Level::Info, "init, sprite loaded");
    }

    fn on_tick(_dt: f32) {
        let clear = Clear {
            r: 20,
            g: 20,
            b: 30,
            a: 255,
        };
        publish(Clear::TOPIC, &clear.encode());
        let draw = DrawSprite {
            id: SPRITE_ID,
            dst_x: DST_X,
            dst_y: DST_Y,
            src_x: 0,
            src_y: 0,
            src_w: FRAME_SIZE,
            src_h: FRAME_SIZE,
        };
        publish(DrawSprite::TOPIC, &draw.encode());
    }

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

export!(Component);
