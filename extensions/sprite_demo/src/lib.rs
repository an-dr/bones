wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, publish, subscribe, Level};
use bones_messages::gfx::{Clear, DrawCircle, DrawRect, DrawSprite, DrawText, LoadSprite, SetCamera};
use bones_messages::{EncodeMessage, Message};

const SPRITE_PNG: &[u8] = include_bytes!("assets/robot_william.png");
const SPRITE_ID: u32 = 1;
// robot_william.png is a 256x64 strip of four 64x64 frames.
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

        // Camera is retained global state (gfx/set-camera), so a one-time,
        // non-identity offset+zoom is enough to prove the renderer applies
        // it — no per-tick republishing needed.
        let camera = SetCamera {
            x: 50.0,
            y: 30.0,
            zoom: 1.5,
        };
        publish(SetCamera::TOPIC, &camera.encode());
    }

    fn on_tick(_dt: f32) {
        let clear = Clear {
            r: 20,
            g: 20,
            b: 30,
            a: 255,
        };
        publish(Clear::TOPIC, &clear.encode());

        // Two overlapping draws on different layers prove composite order
        // is by `layer`, not publish order: frame 2 (layer 1) is published
        // first here but still draws above frame 0 (layer 0). The front
        // draw also exercises rotate/scale/flip/tint together.
        let front = DrawSprite {
            id: SPRITE_ID,
            dst_x: DST_X + 20,
            dst_y: DST_Y + 20,
            dst_w: (FRAME_SIZE as f32 * 1.5) as u32,
            dst_h: (FRAME_SIZE as f32 * 1.5) as u32,
            src_x: (FRAME_SIZE * 2) as i32,
            src_y: 0,
            src_w: FRAME_SIZE,
            src_h: FRAME_SIZE,
            layer: 1,
            angle: 15.0,
            flip_h: true,
            flip_v: false,
            tint: (255, 160, 160, 255),
        };
        publish(DrawSprite::TOPIC, &front.encode());

        let back = DrawSprite {
            id: SPRITE_ID,
            dst_x: DST_X,
            dst_y: DST_Y,
            dst_w: FRAME_SIZE,
            dst_h: FRAME_SIZE,
            src_x: 0,
            src_y: 0,
            src_w: FRAME_SIZE,
            src_h: FRAME_SIZE,
            layer: 0,
            angle: 0.0,
            flip_h: false,
            flip_v: false,
            tint: (255, 255, 255, 255),
        };
        publish(DrawSprite::TOPIC, &back.encode());

        // Shape commands, on the same layer as the background sprite —
        // proves shapes and sprites composite together, not in separate
        // passes.
        let outline = DrawRect {
            x: DST_X - 12,
            y: DST_Y - 12,
            w: FRAME_SIZE + 24,
            h: FRAME_SIZE + 24,
            filled: false,
            color: (80, 200, 255, 255),
            layer: 0,
        };
        publish(DrawRect::TOPIC, &outline.encode());

        let marker = DrawCircle {
            x: DST_X + FRAME_SIZE as i32 + 40,
            y: DST_Y + FRAME_SIZE as i32 / 2,
            radius: 18,
            filled: true,
            color: (255, 220, 60, 255),
            layer: 2,
        };
        publish(DrawCircle::TOPIC, &marker.encode());

        // Nameplate text, world-space like everything else here (the
        // camera transform applies uniformly — no separate screen-space
        // HUD layer exists yet), on the topmost layer so it's never
        // occluded by the shapes or sprites below it.
        let nameplate = DrawText {
            text: "William",
            x: DST_X - 8,
            y: DST_Y - 34,
            size: 14,
            color: (255, 255, 255, 255),
            layer: 3,
        };
        publish(DrawText::TOPIC, &nameplate.encode());
    }

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

export!(Component);
