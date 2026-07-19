wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use bones::core::host_api::{log, publish, Level};
use bones_messages::game_core::{LoadTilemap, SpawnEntity};
use bones_messages::gfx::LoadSprite;
use bones_messages::{EncodeMessage, Message};

const LEVEL_TMX: &[u8] = include_bytes!("assets/level.tmx");
const SPRITE_PNG: &[u8] = include_bytes!("assets/robot_william.png");
const SPRITE_ID: u32 = 1;
// robot_william.png is a 256x64 strip of four 64x64 frames.
const FRAME_SIZE: u32 = 64;

struct Component;

impl Guest for Component {
    fn init() {
        // No subscriptions: this demo only drives game-core at startup and
        // lets its own core/tick simulation (and gfx publishing) run from
        // there — the same "load once, then let the module tick itself"
        // shape sprite_demo's camera setup already uses.
        let load_sprite = LoadSprite { id: SPRITE_ID, png_bytes: SPRITE_PNG };
        publish(LoadSprite::TOPIC, &load_sprite.encode());

        let load_tilemap = LoadTilemap { tmx_bytes: LEVEL_TMX };
        publish(LoadTilemap::TOPIC, &load_tilemap.encode());

        // Two overlapping dynamic entities, both above the level's floor
        // collider (a 128x16 rect at y=64..80) — game-core's rapier2d step
        // separates them from each other and rests them on the floor,
        // while each keeps animating through its 4 sprite frames.
        let left = SpawnEntity {
            sprite_id: SPRITE_ID,
            x: 40.0,
            y: 40.0,
            frame_w: FRAME_SIZE,
            frame_h: FRAME_SIZE,
            frame_count: 4,
            frame_duration: 0.15,
            collider_half_w: FRAME_SIZE as f32 / 2.0,
            collider_half_h: FRAME_SIZE as f32 / 2.0,
        };
        publish(SpawnEntity::TOPIC, &left.encode());

        let right = SpawnEntity { x: 70.0, ..left };
        publish(SpawnEntity::TOPIC, &right.encode());

        log(Level::Info, "init: tilemap + sprite loaded, two colliding entities spawned");
    }

    fn on_tick(_dt: f32) {}

    fn on_message(_topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        None
    }
}

export!(Component);
