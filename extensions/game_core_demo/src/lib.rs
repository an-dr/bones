wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use std::cell::RefCell;

use bones::core::host_api::{log, publish, subscribe, Level};
use bones_messages::game_core::{BodyKind, EntityOp, EntityOpMessage, LoadTilemap, Sprite};
use bones_messages::gfx::LoadSprite;
use bones_messages::input::{GamepadAxis, KeyDown, KeyUp};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

const LEVEL_TMX: &[u8] = include_bytes!("assets/level.tmx");
const SPRITE_PNG: &[u8] = include_bytes!("assets/robot_william.png");
const SPRITE_ID: u32 = 1;
// robot_william.png is a 256x64 strip of four 64x64 frames.
const FRAME_SIZE: u32 = 64;
const CONTROLLED_ENTITY_ID: u32 = 1;
const OBSTACLE_HALF_EXTENT: f32 = 24.0;
const OBSTACLE_COLOR: (u8, u8, u8, u8) = (200, 60, 60, 255);
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

fn publish_entity_op(op: EntityOp) {
    publish(EntityOpMessage::TOPIC, &EntityOpMessage(op).encode());
}

/// A plain colored square, no sprite — obstacles and walls don't need art
/// (only the controlled entity uses `robot_william.png`).
fn spawn_obstacle(entity_id: u32, x: f32, y: f32) {
    publish_entity_op(EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: None,
        square_color: OBSTACLE_COLOR,
        collider_half_w: OBSTACLE_HALF_EXTENT,
        collider_half_h: OBSTACLE_HALF_EXTENT,
        body_kind: BodyKind::Dynamic,
    });
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

        // The controlled entity: driven by set-velocity from on_tick, below
        // — the only entity in this demo that uses the robot sprite.
        publish_entity_op(EntityOp::Spawn {
            entity_id: CONTROLLED_ENTITY_ID,
            x: 60.0,
            y: 60.0,
            sprite: Some(Sprite {
                sprite_id: SPRITE_ID,
                frame_w: FRAME_SIZE,
                frame_h: FRAME_SIZE,
                frame_count: 4,
                frame_duration: 0.15,
            }),
            square_color: (0, 0, 0, 0),
            collider_half_w: FRAME_SIZE as f32 / 2.0,
            collider_half_h: FRAME_SIZE as f32 / 2.0,
            body_kind: BodyKind::Dynamic,
        });

        // Several stationary obstacle squares scattered around the interior
        // walls (level.tmx's cross-shaped Collision layer) — the controlled
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
        publish_entity_op(EntityOp::SetVelocity {
            entity_id: CONTROLLED_ENTITY_ID,
            vx,
            vy,
        });
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
