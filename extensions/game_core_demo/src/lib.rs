wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use std::cell::RefCell;

use bones::core::host_api::{log, publish, subscribe, Level};
use bones_messages::game_core::{LoadTilemap, SetVelocity, SpawnEntity};
use bones_messages::gfx::LoadSprite;
use bones_messages::input::{GamepadAxis, KeyDown, KeyUp};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

const LEVEL_TMX: &[u8] = include_bytes!("assets/level.tmx");
const SPRITE_PNG: &[u8] = include_bytes!("assets/robot_william.png");
const SPRITE_ID: u32 = 1;
// robot_william.png is a 256x64 strip of four 64x64 frames.
const FRAME_SIZE: u32 = 64;
const CONTROLLED_ENTITY_ID: u32 = 1;
const MOVE_SPEED: f32 = 120.0;
// This demo's own dead zone: below this, stick drift (platform reports raw
// axis values with no dead zone applied) shouldn't move the entity.
const AXIS_DEAD_ZONE: f32 = 0.15;

// `Held` stays with `Component` rather than splitting further: it's purely
// this extension's own thread-local input state, never constructed or
// named outside this file, never meaningful on its own.
#[derive(Default)]
struct Held {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    // Left-stick axes, last value seen; independent of the WASD keys so
    // whichever input moved last still wins its own axis on release.
    axis_x: f32,
    axis_y: f32,
}

impl Held {
    fn velocity(&self) -> (f32, f32) {
        let key_x = (self.right as i32 - self.left as i32) as f32;
        let key_y = (self.down as i32 - self.up as i32) as f32;
        // Whichever source has a stronger signal on each axis wins — lets a
        // gamepad and keyboard coexist without one silently overriding the
        // other at zero.
        let x = if key_x != 0.0 {
            key_x
        } else {
            deadzone(self.axis_x)
        };
        let y = if key_y != 0.0 {
            key_y
        } else {
            deadzone(self.axis_y)
        };
        (x * MOVE_SPEED, y * MOVE_SPEED)
    }
}

fn deadzone(value: f32) -> f32 {
    if value.abs() < AXIS_DEAD_ZONE {
        0.0
    } else {
        value
    }
}

thread_local! {
    static HELD: RefCell<Held> = RefCell::new(Held::default());
}

fn spawn_obstacle(entity_id: u32, x: f32, y: f32) {
    let obstacle = SpawnEntity {
        entity_id,
        sprite_id: SPRITE_ID,
        x,
        y,
        frame_w: FRAME_SIZE,
        frame_h: FRAME_SIZE,
        frame_count: 1,
        frame_duration: 0.0,
        collider_half_w: FRAME_SIZE as f32 / 2.0,
        collider_half_h: FRAME_SIZE as f32 / 2.0,
    };
    publish(SpawnEntity::TOPIC, &obstacle.encode());
}

struct Component;

impl Guest for Component {
    fn init() {
        subscribe(KeyDown::TOPIC);
        subscribe(KeyUp::TOPIC);
        subscribe(GamepadAxis::TOPIC);
        subscribe("core/tick");

        let load_sprite = LoadSprite {
            id: SPRITE_ID,
            png_bytes: SPRITE_PNG,
        };
        publish(LoadSprite::TOPIC, &load_sprite.encode());

        let load_tilemap = LoadTilemap {
            tmx_bytes: LEVEL_TMX,
        };
        publish(LoadTilemap::TOPIC, &load_tilemap.encode());

        // The controlled entity: driven by set-velocity from on_tick, below.
        let controlled = SpawnEntity {
            entity_id: CONTROLLED_ENTITY_ID,
            sprite_id: SPRITE_ID,
            x: 60.0,
            y: 60.0,
            frame_w: FRAME_SIZE,
            frame_h: FRAME_SIZE,
            frame_count: 4,
            frame_duration: 0.15,
            collider_half_w: FRAME_SIZE as f32 / 2.0,
            collider_half_h: FRAME_SIZE as f32 / 2.0,
        };
        publish(SpawnEntity::TOPIC, &controlled.encode());

        // Several stationary obstacles scattered around the interior walls
        // (level.tmx's cross-shaped Collision layer) — the controlled
        // entity visibly stops against each one, proving entity-entity
        // collision alongside the tilemap's entity-terrain collision.
        spawn_obstacle(2, 350.0, 60.0);
        spawn_obstacle(3, 350.0, 160.0);
        spawn_obstacle(4, 350.0, 260.0);
        spawn_obstacle(5, 120.0, 260.0);

        log(Level::Info, "init: tilemap + sprite loaded; WASD/gamepad-left-stick moves the entity into obstacles");
    }

    fn on_tick(_dt: f32) {
        let (vx, vy) = HELD.with(|held| held.borrow().velocity());
        let set = SetVelocity {
            entity_id: CONTROLLED_ENTITY_ID,
            vx,
            vy,
        };
        publish(SetVelocity::TOPIC, &set.encode());
    }

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        match topic.as_str() {
            KeyDown::TOPIC => {
                if let Ok(message) = KeyDown::decode(&payload) {
                    set_key_held(message.key, true);
                }
            }
            KeyUp::TOPIC => {
                if let Ok(message) = KeyUp::decode(&payload) {
                    set_key_held(message.key, false);
                }
            }
            GamepadAxis::TOPIC => {
                if let Ok(message) = GamepadAxis::decode(&payload) {
                    HELD.with(|held| {
                        let mut held = held.borrow_mut();
                        match message.axis {
                            "LeftX" => held.axis_x = message.value,
                            "LeftY" => held.axis_y = message.value,
                            _ => {}
                        }
                    });
                }
            }
            _ => {}
        }
        None
    }
}

fn set_key_held(key: &str, is_down: bool) {
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        match key {
            "W" => held.up = is_down,
            "S" => held.down = is_down,
            "A" => held.left = is_down,
            "D" => held.right = is_down,
            _ => {}
        }
    });
}

export!(Component);
