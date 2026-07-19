wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use std::cell::RefCell;
use std::collections::HashMap;

use bones::core::host_api::{log, publish, subscribe, Level};
use bones_messages::audio::{LoadSound, PlaySound};
use bones_messages::game_core::{
    BodyKind, Collision, EntityOp, EntityOpMessage, LoadTilemap, Sprite,
};
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
const FLASH_COLOR: (u8, u8, u8, u8) = (255, 255, 255, 255);
// 0.3s: within the requested 0.1-0.5s hit-flash window.
const FLASH_DURATION_SECONDS: f32 = 0.3;
const MOVER_HALF_EXTENT: f32 = 16.0;
const MOVER_COLOR: (u8, u8, u8, u8) = (60, 120, 220, 255);
const MOVE_SPEED: f32 = 120.0;
// This demo's own dead zone: below this, stick drift (platform reports raw
// axis values with no dead zone applied) shouldn't move the entity.
const AXIS_DEAD_ZONE: f32 = 0.15;
const SAMPLE_RATE: u32 = 44_100;
const FOOTSTEP_SOUND_ID: u32 = 1;
const HIT_SOUND_ID: u32 = 2;
// Roughly two steps per second while moving.
const FOOTSTEP_INTERVAL_SECONDS: f32 = 0.5;

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

// `State` stays with `Component` rather than splitting further: it's purely
// this extension's own thread-local store, never constructed or named
// outside this file, never meaningful on its own.
#[derive(Default)]
struct State {
    held: Held,
    // Seconds until the footstep sound may play again; only counts down
    // while actually moving, so it's always ready the instant movement
    // starts rather than carrying over stale cooldown from before a stop.
    footstep_cooldown: f32,
    // Obstacle entity_id -> seconds remaining before its flash reverts.
    // Only obstacle entities (never the controlled sprite) ever appear
    // here — a hit always flashes back to OBSTACLE_COLOR, which every
    // obstacle in this demo shares.
    flashing: HashMap<u32, f32>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn publish_entity_op(op: EntityOp) {
    publish(EntityOpMessage::TOPIC, &EntityOpMessage(op).encode());
}

/// Synthesizes a mono 16-bit PCM WAV sine tone over an exact whole number
/// of `cycles` of `frequency_hz` — the waveform then starts and ends at a
/// zero crossing, so a one-shot sound has no end-of-buffer click. Avoids
/// embedding a licensed audio asset just for a demo (same approach
/// `extensions/audio_demo` already uses).
fn synthesize_tone(frequency_hz: f32, cycles: u32, amplitude: f32) -> Vec<u8> {
    let sample_count = ((cycles as f32 / frequency_hz) * SAMPLE_RATE as f32).round() as u32;
    let mut samples = Vec::with_capacity(sample_count as usize);
    for n in 0..sample_count {
        let t = n as f32 / SAMPLE_RATE as f32;
        let value = (amplitude
            * (2.0 * core::f32::consts::PI * frequency_hz * t).sin()
            * i16::MAX as f32) as i16;
        samples.push(value);
    }

    let data_bytes = samples.len() * 2;
    let mut wav = Vec::with_capacity(44 + data_bytes);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
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

/// A blue square with no inertia: `Kinematic`, so it's solid (blocks and
/// pushes the `Dynamic` obstacles/robot) but nothing can push it back —
/// stays exactly where spawned unless a future `SetVelocity` moves it.
fn spawn_mover(entity_id: u32, x: f32, y: f32) {
    publish_entity_op(EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: None,
        square_color: MOVER_COLOR,
        collider_half_w: MOVER_HALF_EXTENT,
        collider_half_h: MOVER_HALF_EXTENT,
        body_kind: BodyKind::Kinematic,
    });
}

struct Component;

impl Guest for Component {
    fn init() {
        subscribe(KeyDown::TOPIC);
        subscribe(KeyUp::TOPIC);
        subscribe(GamepadAxis::TOPIC);
        subscribe(Collision::TOPIC);
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

        // ~80ms, 1200Hz — a short click for footsteps.
        let footstep = synthesize_tone(1200.0, 96, 0.5);
        publish(
            LoadSound::TOPIC,
            &LoadSound {
                id: FOOTSTEP_SOUND_ID,
                bytes: &footstep,
            }
            .encode(),
        );
        // ~120ms, 300Hz — a lower blip for a hit, distinct from footsteps.
        let hit = synthesize_tone(300.0, 36, 0.7);
        publish(
            LoadSound::TOPIC,
            &LoadSound {
                id: HIT_SOUND_ID,
                bytes: &hit,
            }
            .encode(),
        );

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

        // Several stationary red obstacle squares — the controlled entity
        // (and, if pushed into a neighbor, one obstacle into another)
        // visibly stops against each one, proving entity-entity collision
        // alongside the tilemap's entity-terrain collision. A hit between
        // two of these flashes white briefly and plays a sound (see
        // on_message's Collision handling).
        spawn_obstacle(2, 350.0, 60.0);
        spawn_obstacle(3, 350.0, 160.0);
        spawn_obstacle(4, 350.0, 260.0);
        spawn_obstacle(5, 120.0, 260.0);

        // Two blue Kinematic squares: solid and immovable — pushing the
        // robot into one stops the robot without the square ever budging,
        // unlike the red Dynamic obstacles above.
        spawn_mover(6, 220.0, 60.0);
        spawn_mover(7, 220.0, 260.0);

        log(
            Level::Info,
            "init: tilemap + sprite loaded; WASD/gamepad-left-stick moves the entity into obstacles",
        );
    }

    fn on_tick(dt: f32) {
        let (vx, vy) = STATE.with(|state| state.borrow().held.velocity());
        publish_entity_op(EntityOp::SetVelocity {
            entity_id: CONTROLLED_ENTITY_ID,
            vx,
            vy,
        });

        let moving = vx != 0.0 || vy != 0.0;
        STATE.with(|state| {
            let mut state = state.borrow_mut();

            // Footstep sound: only while actually moving, at most once per
            // FOOTSTEP_INTERVAL_SECONDS — never while stationary, matching
            // game-core's own velocity-gated animation for the same entity.
            if moving {
                state.footstep_cooldown -= dt;
                if state.footstep_cooldown <= 0.0 {
                    state.footstep_cooldown = FOOTSTEP_INTERVAL_SECONDS;
                    publish(
                        PlaySound::TOPIC,
                        &PlaySound {
                            id: FOOTSTEP_SOUND_ID,
                            volume: 0.5,
                        }
                        .encode(),
                    );
                }
            } else {
                state.footstep_cooldown = 0.0;
            }

            // Hit-flash countdown: revert any obstacle whose flash timer
            // has expired back to its normal color.
            state.flashing.retain(|&entity_id, remaining| {
                *remaining -= dt;
                if *remaining > 0.0 {
                    true
                } else {
                    publish_entity_op(EntityOp::SetColor {
                        entity_id,
                        color: OBSTACLE_COLOR,
                    });
                    false
                }
            });
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
                    STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        match message.axis {
                            "LeftX" => state.held.axis_x = message.value,
                            "LeftY" => state.held.axis_y = message.value,
                            _ => {}
                        }
                    });
                }
            }
            Collision::TOPIC => {
                if let Ok(message) = Collision::decode(&payload) {
                    on_collision(message);
                }
            }
            _ => {}
        }
        None
    }
}

/// Obstacle ids are 2-5 (see `init`); the controlled entity (1) and the
/// Kinematic movers (6-7) never flash — only a red-obstacle-on-red-obstacle
/// hit counts as "damage" for this demo.
fn is_obstacle(entity_id: u32) -> bool {
    (2..=5).contains(&entity_id)
}

fn on_collision(collision: Collision) {
    if !is_obstacle(collision.entity_id_a) || !is_obstacle(collision.entity_id_b) {
        return;
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        for entity_id in [collision.entity_id_a, collision.entity_id_b] {
            state.flashing.insert(entity_id, FLASH_DURATION_SECONDS);
        }
    });
    publish_entity_op(EntityOp::SetColor {
        entity_id: collision.entity_id_a,
        color: FLASH_COLOR,
    });
    publish_entity_op(EntityOp::SetColor {
        entity_id: collision.entity_id_b,
        color: FLASH_COLOR,
    });
    publish(
        PlaySound::TOPIC,
        &PlaySound {
            id: HIT_SOUND_ID,
            volume: 0.9,
        }
        .encode(),
    );
}

fn set_key_held(key: &str, is_down: bool) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        match key {
            "W" => state.held.up = is_down,
            "S" => state.held.down = is_down,
            "A" => state.held.left = is_down,
            "D" => state.held.right = is_down,
            _ => {}
        }
    });
}

export!(Component);
