wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use std::cell::RefCell;
use std::collections::HashMap;

use bones::core::host_api::{log, publish, subscribe, Level};
use bones_messages::audio::{LoadSound, PlaySound};
use bones_messages::game_core::{
    BodyKind, Collision, EntityOp, EntityOpMessage, LoadTilemap, PhysicsWorlds, Shape, Sprite,
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
// Narrower than the sprite frame (64px) — the robot's actual body/screen
// width, not its full drawn frame including empty margin either side.
const CONTROLLED_HALF_EXTENT: f32 = 16.0;
const BIG_BOX_HALF_EXTENT: f32 = 24.0;
// Purple: red is reserved for the hazard triangles below, so a big box
// can't be visually confused with a life-costing hazard.
const BIG_BOX_COLOR: (u8, u8, u8, u8) = (150, 60, 200, 255);
const FLASH_COLOR: (u8, u8, u8, u8) = (255, 255, 255, 255);
// 0.3s: within the requested 0.1-0.5s hit-flash window.
const FLASH_DURATION_SECONDS: f32 = 0.3;
const SMALL_BOX_HALF_EXTENT: f32 = 16.0;
const SMALL_BOX_COLOR: (u8, u8, u8, u8) = (60, 120, 220, 255);
const HAZARD_HALF_EXTENT: f32 = 20.0;
const HAZARD_COLOR: (u8, u8, u8, u8) = (200, 60, 60, 255);
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
    // Big box/hazard entity_id -> seconds remaining before its flash
    // reverts (never the controlled sprite, which has no SquareColor to
    // flash). Reverts to `BIG_BOX_COLOR` or `HAZARD_COLOR` depending on
    // which kind of entity it names — see `revert_color`.
    flashing: HashMap<u32, f32>,
    // Local mirror of game-core's own EntityOp::SetDebugHitboxes toggle —
    // tracked here (rather than reading it back) so pressing H can flip
    // it without this extension needing any state from game-core itself.
    debug_hitboxes: bool,
    score: u32,
    // Starts at FULL_LIFE (see init); never below zero.
    life: u32,
}

const FULL_LIFE: u32 = 3;

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

/// A plain colored square, no sprite — big boxes and walls don't need art
/// (only the controlled entity uses `robot_william.png`). Registered in
/// the `rapier2d` world only (ADR-021, `PhysicsWorlds::RAPIER2D` — the
/// default, stated explicitly here since this demo now exercises more
/// than one world). Score's collision trigger (see `on_collision`).
fn spawn_big_box(entity_id: u32, x: f32, y: f32) {
    publish_entity_op(EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: None,
        square_color: BIG_BOX_COLOR,
        shape: Shape::Rect,
        collider_half_w: BIG_BOX_HALF_EXTENT,
        collider_half_h: BIG_BOX_HALF_EXTENT,
        body_kind: BodyKind::Dynamic,
        worlds: PhysicsWorlds::RAPIER2D,
    });
}

/// A blue square with no inertia: `Frictionless`, so the robot or a big
/// box can push it around like any other body, but it carries no
/// momentum of its own — it stops the instant nothing is pushing it,
/// instead of coasting or drifting. Registered in the `retro` world only
/// (ADR-021, `PhysicsWorlds::RETRO`) — this demo's example of the
/// no-mass, no-solver backend.
fn spawn_small_box(entity_id: u32, x: f32, y: f32) {
    publish_entity_op(EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: None,
        square_color: SMALL_BOX_COLOR,
        shape: Shape::Rect,
        collider_half_w: SMALL_BOX_HALF_EXTENT,
        collider_half_h: SMALL_BOX_HALF_EXTENT,
        body_kind: BodyKind::Frictionless,
        worlds: PhysicsWorlds::RETRO,
    });
}

/// A red triangle: costs the robot life on contact (see `on_collision`).
/// `Kinematic` and never given a `SetVelocity` — game-core's "moves
/// exactly as commanded, never itself pushed" body type is this demo's
/// way to spawn a stationary hazard the robot can't shove around, since
/// the wire vocabulary has no dedicated immovable-but-not-tilemap kind.
/// Registered in the `rapier2d` world only: `Shape::Triangle` is a real
/// collider shape there, but only an AABB-bounding-box approximation in
/// `retro` (see `physics::RetroBackend`'s own docs) — not worth spawning
/// into a world where it wouldn't actually be triangular.
fn spawn_hazard(entity_id: u32, x: f32, y: f32) {
    publish_entity_op(EntityOp::Spawn {
        entity_id,
        x,
        y,
        sprite: None,
        square_color: HAZARD_COLOR,
        shape: Shape::Triangle,
        collider_half_w: HAZARD_HALF_EXTENT,
        collider_half_h: HAZARD_HALF_EXTENT,
        body_kind: BodyKind::Kinematic,
        worlds: PhysicsWorlds::RAPIER2D,
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
        // — the only entity in this demo that uses the robot sprite. Its
        // collider is narrower than the sprite frame (CONTROLLED_HALF_EXTENT
        // vs. FRAME_SIZE/2.0) — the robot's actual body width, not its full
        // drawn frame including empty margin either side. Registered in
        // both physics worlds at once (ADR-021, `PhysicsWorlds::BOTH`) —
        // this demo's example of a single entity genuinely simulated by two
        // independent backends simultaneously; `retro` outranks `rapier2d`
        // in `PhysicsWorldKind::PRIORITY`, so the robot's drawn position
        // tracks the no-mass, no-solver world while its rapier2d copy
        // (still pushed by/pushing big boxes) is snapped to match every
        // tick.
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
            shape: Shape::Rect,
            collider_half_w: CONTROLLED_HALF_EXTENT,
            collider_half_h: CONTROLLED_HALF_EXTENT,
            body_kind: BodyKind::Dynamic,
            worlds: PhysicsWorlds::BOTH,
        });

        // Several stationary purple big-box squares — the controlled entity
        // (and, if pushed into a neighbor, one box into another) visibly
        // stops against each one, proving entity-entity collision alongside
        // the tilemap's entity-terrain collision. A hit between two of
        // these flashes white briefly and plays a sound; the robot hitting
        // one scores a point (see on_message's Collision handling).
        spawn_big_box(2, 350.0, 60.0);
        spawn_big_box(3, 350.0, 160.0);
        spawn_big_box(4, 350.0, 260.0);
        spawn_big_box(5, 120.0, 260.0);

        // Two blue Frictionless squares: pushable, but carry no momentum —
        // pushing the robot into one moves it, but it stops the instant
        // contact ends, unlike the purple Dynamic big boxes above.
        spawn_small_box(6, 220.0, 60.0);
        spawn_small_box(7, 220.0, 260.0);

        // Two stationary red hazard triangles — the robot loses life on
        // contact (see on_message's Collision handling). Positioned clear
        // of every other spawn above.
        spawn_hazard(8, 60.0, 160.0);
        spawn_hazard(9, 400.0, 200.0);

        STATE.with(|state| state.borrow_mut().life = FULL_LIFE);

        log(
            Level::Info,
            "init: tilemap + sprite loaded; WASD/gamepad-left-stick moves the entity into big/small boxes and hazards; H toggles hitbox outlines",
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

            // Hit-flash countdown: revert any big box/hazard whose flash
            // timer has expired back to its normal color.
            state.flashing.retain(|&entity_id, remaining| {
                *remaining -= dt;
                if *remaining > 0.0 {
                    true
                } else {
                    publish_entity_op(EntityOp::SetColor {
                        entity_id,
                        color: revert_color(entity_id),
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
                    if message.key == "H" {
                        toggle_debug_hitboxes();
                    } else {
                        set_key_held(message.key, true);
                    }
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

/// Big box ids are 2-5 (see `init`).
fn is_big_box(entity_id: u32) -> bool {
    (2..=5).contains(&entity_id)
}

/// Hazard triangle ids are 8-9 (see `init`).
fn is_hazard(entity_id: u32) -> bool {
    (8..=9).contains(&entity_id)
}

/// The color a flashed entity reverts to once its flash timer expires —
/// only ever called with a big-box or hazard id (the only entities
/// `on_collision` ever flashes).
fn revert_color(entity_id: u32) -> (u8, u8, u8, u8) {
    if is_hazard(entity_id) {
        HAZARD_COLOR
    } else {
        BIG_BOX_COLOR
    }
}

/// Three collision shapes count here (small boxes, the tilemap, and any
/// other pairing are silently ignored): two big boxes clashing (flashes
/// both, sound, no score/life change), the robot hitting a big box
/// (flashes it, sound, score += 1 — the original "red box collision"
/// trigger, now purple), and the robot hitting a hazard triangle (flashes
/// it, sound, life -= 1, saturating at zero rather than wrapping).
fn on_collision(collision: Collision) {
    let (a, b) = (collision.entity_id_a, collision.entity_id_b);

    // `robot_hit` names the non-robot side of a robot collision, `None`
    // for a big-box-on-big-box clash (score/life untouched either way).
    let (flash_targets, robot_hit): (Vec<u32>, Option<u32>) = if is_big_box(a) && is_big_box(b) {
        (vec![a, b], None)
    } else if a == CONTROLLED_ENTITY_ID && (is_big_box(b) || is_hazard(b)) {
        (vec![b], Some(b))
    } else if b == CONTROLLED_ENTITY_ID && (is_big_box(a) || is_hazard(a)) {
        (vec![a], Some(a))
    } else {
        return;
    };

    if let Some(hit) = robot_hit {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if is_big_box(hit) {
                state.score += 1;
            } else {
                state.life = state.life.saturating_sub(1);
            }
            log(
                Level::Info,
                &format!("score={} life={}", state.score, state.life),
            );
        });
    }

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        for &entity_id in &flash_targets {
            state.flashing.insert(entity_id, FLASH_DURATION_SECONDS);
        }
    });
    for &entity_id in &flash_targets {
        publish_entity_op(EntityOp::SetColor {
            entity_id,
            color: FLASH_COLOR,
        });
    }
    publish(
        PlaySound::TOPIC,
        &PlaySound {
            id: HIT_SOUND_ID,
            volume: 0.9,
        }
        .encode(),
    );
}

fn toggle_debug_hitboxes() {
    let enabled = STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.debug_hitboxes = !state.debug_hitboxes;
        state.debug_hitboxes
    });
    publish_entity_op(EntityOp::SetDebugHitboxes { enabled });
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
