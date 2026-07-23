wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use std::cell::RefCell;
use std::collections::HashMap;

use bones::core::host_api::{log, publish, request_exit, subscribe, Level};
use bones_messages::audio::{LoadSound, PlaySound};
use bones_messages::game_core::{
    BodyKind, Collision, EntityOp, EntityOpMessage, LoadTilemap, PhysicsWorlds, Shape, Sprite,
    TilesetImage,
};
use bones_messages::gfx::{DrawRect, DrawText, LoadSprite};
use bones_messages::input::{GamepadAxis, KeyDown, KeyUp, MouseDown};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

const LEVEL_TMX: &[u8] = include_bytes!("assets/level.tmx");
const SPRITE_PNG: &[u8] = include_bytes!("assets/robot_william.png");
const SPRITE_ID: u32 = 1;
// robot_william.png is a 256x64 strip of four 64x64 frames.
const FRAME_SIZE: u32 = 64;

// Ground tile art: "Pixel Art Top Down - Basic" (itch.io/Unity Asset
// Store), the user's own asset pack from a sibling project - not
// originated in this repo, license terms not verified here. Tile
// placement itself lives in level.tmx's "Ground" layer (real Tiled data,
// parsed and rendered by game-core via the `tiled` crate) — this
// extension only supplies the tileset images and the sprite ids to
// register them under, matched to the .tmx's embedded `<tileset name=...>`
// by name.
const TILESET_GRASS_PNG: &[u8] = include_bytes!("assets/tileset_grass.png");
const TILESET_BRICKS_PNG: &[u8] = include_bytes!("assets/tileset_bricks.png");
const GRASS_SPRITE_ID: u32 = 2;
const BRICK_SPRITE_ID: u32 = 3;
const GRASS_TILESET_NAME: &str = "grass";
const BRICK_TILESET_NAME: &str = "bricks";

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
const BIG_BOX_IDS: [u32; 4] = [2, 3, 4, 5];
const SMALL_BOX_IDS: [u32; 2] = [6, 7];
// level.tmx's walled interior sits at this offset from the map's own
// origin (a 64px grass margin surrounds it on every side) — every spawn
// coordinate below is the original design's coordinate (when the wall
// sat directly at the map edge) plus this same offset, so the whole
// layout shifted as one fixed unit rather than being redesigned.
const LEVEL_ORIGIN_X: f32 = 64.0;
const LEVEL_ORIGIN_Y: f32 = 64.0;
const MOVE_SPEED: f32 = 120.0;
// This demo's own dead zone: below this, stick drift (platform reports raw
// axis values with no dead zone applied) shouldn't move the entity.
const AXIS_DEAD_ZONE: f32 = 0.15;
const SAMPLE_RATE: u32 = 44_100;
const FOOTSTEP_SOUND_ID: u32 = 1;
const HIT_SOUND_ID: u32 = 2;
// Roughly two steps per second while moving.
const FOOTSTEP_INTERVAL_SECONDS: f32 = 0.2;

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

/// The in-game pause menu Esc opens/closes. `Settings` has its own Back
/// button to return to `Main` — Esc always toggles `Closed` <-> whatever
/// it currently is, not step-by-step through `Settings` first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum MenuState {
    #[default]
    Closed,
    Main,
    Settings,
}

const BUTTON_SETTINGS: u32 = 1;
const BUTTON_EXIT: u32 = 2;
const BUTTON_BACK: u32 = 3;
// Presets start well clear of the fixed ids above; big-box and small-box
// ranges are spaced far enough apart (10 ids each) that neither preset
// list could ever grow into the other's range unnoticed.
const BUTTON_BIG_BOX_PRESET_BASE: u32 = 10;
const BUTTON_SMALL_BOX_PRESET_BASE: u32 = 20;

type Preset = (&'static str, (u8, u8, u8, u8));

// Warm, excluding red: red is the hazard triangles' color, so a big box
// staying warm-but-not-red never reads as a hazard.
const BIG_BOX_PRESETS: [Preset; 3] = [
    ("Orange", (255, 140, 0, 255)),
    ("Yellow", (230, 200, 30, 255)),
    ("Pink", (230, 60, 140, 255)),
];
// Cold.
const SMALL_BOX_PRESETS: [Preset; 3] = [
    ("Cyan", (40, 200, 200, 255)),
    ("Teal", (30, 150, 120, 255)),
    ("Indigo", (90, 60, 200, 255)),
];

/// `id`'s preset color within `presets`, addressed starting at `base` (one
/// button id per preset, in array order) — `None` if `id` doesn't fall in
/// that range at all.
fn preset_color(id: u32, base: u32, presets: &[Preset]) -> Option<(u8, u8, u8, u8)> {
    let index = id.checked_sub(base)? as usize;
    presets.get(index).map(|&(_, color)| color)
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
    // flash). Reverts to the live `big_box_color` or `HAZARD_COLOR`
    // depending on which kind of entity it names — see `revert_color`.
    flashing: HashMap<u32, f32>,
    // Local mirror of game-core's own EntityOp::SetDebugHitboxes toggle —
    // tracked here (rather than reading it back) so pressing H can flip
    // it without this extension needing any state from game-core itself.
    debug_hitboxes: bool,
    score: u32,
    // Starts at FULL_LIFE (see init); never below zero.
    life: u32,
    menu: MenuState,
    // Live current color for every big/small box, changed by a Settings
    // preset click and applied to every existing entity of that kind at
    // once. Starts at BIG_BOX_COLOR/SMALL_BOX_COLOR (see init) — not
    // derivable from Default, so those two constants stay the source of
    // truth for the initial appearance.
    big_box_color: (u8, u8, u8, u8),
    small_box_color: (u8, u8, u8, u8),
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

// Matches bones.toml's default window size (Config::default) — this demo
// never overrides window_width/window_height, and no engine message
// exposes the actual window size to an extension yet, so this is a fixed
// assumption rather than something queried at runtime. Also what's passed
// as EntityOp::SetCameraFollow's viewport_w/viewport_h, below.
const SCREEN_WIDTH: i32 = 800;
const SCREEN_HEIGHT: i32 = 600;

// HUD/menu draws are all screen_space: true (gfx::DrawRect/DrawText), so
// these fixed pixel positions stay put on screen regardless of where
// EntityOp::SetCameraFollow has panned the camera.
const HUD_LAYER: u8 = 5;
const MENU_LAYER: u8 = 6;

const HUD_X: i32 = 8;
const HUD_Y: i32 = 8;
const HUD_W: u32 = 170;
const HUD_H: u32 = 54;
const HUD_BG_COLOR: (u8, u8, u8, u8) = (15, 15, 20, 255);
const HUD_BORDER_COLOR: (u8, u8, u8, u8) = (90, 90, 110, 255);
const HUD_TEXT_COLOR: (u8, u8, u8, u8) = (235, 235, 235, 255);

/// Draws the always-visible score/life panel directly via `gfx/*` — a
/// solid-color backdrop rect, a thin border, and two text lines. Same
/// "publish it every tick, no retained state" contract game-core's own
/// entity draws already use; this is this demo's own HUD layer, above
/// every entity (`ENTITY_LAYER` is 0 in game-core).
fn draw_hud(score: u32, life: u32) {
    publish(
        DrawRect::TOPIC,
        &DrawRect {
            x: HUD_X,
            y: HUD_Y,
            w: HUD_W,
            h: HUD_H,
            filled: true,
            color: HUD_BG_COLOR,
            layer: HUD_LAYER,
            screen_space: true,
        }
        .encode(),
    );
    publish(
        DrawRect::TOPIC,
        &DrawRect {
            x: HUD_X,
            y: HUD_Y,
            w: HUD_W,
            h: HUD_H,
            filled: false,
            color: HUD_BORDER_COLOR,
            layer: HUD_LAYER,
            screen_space: true,
        }
        .encode(),
    );
    let score_text = format!("Score: {score}");
    publish(
        DrawText::TOPIC,
        &DrawText {
            text: &score_text,
            x: HUD_X + 10,
            y: HUD_Y + 8,
            size: 16,
            color: HUD_TEXT_COLOR,
            layer: HUD_LAYER,
            screen_space: true,
        }
        .encode(),
    );
    let life_text = format!("Life: {life}");
    publish(
        DrawText::TOPIC,
        &DrawText {
            text: &life_text,
            x: HUD_X + 10,
            y: HUD_Y + 30,
            size: 16,
            color: HUD_TEXT_COLOR,
            layer: HUD_LAYER,
            screen_space: true,
        }
        .encode(),
    );
}

const PANEL_W: u32 = 360;
const PANEL_H: u32 = 300;
const PANEL_X: i32 = (SCREEN_WIDTH - PANEL_W as i32) / 2;
const PANEL_Y: i32 = 130;
const PANEL_BG_COLOR: (u8, u8, u8, u8) = (20, 20, 30, 255);
const PANEL_BORDER_COLOR: (u8, u8, u8, u8) = (110, 110, 140, 255);
const PANEL_TITLE_COLOR: (u8, u8, u8, u8) = (255, 255, 255, 255);
const SECTION_LABEL_COLOR: (u8, u8, u8, u8) = (200, 200, 210, 255);
const BUTTON_COLOR: (u8, u8, u8, u8) = (70, 90, 140, 255);
const BUTTON_TEXT_COLOR: (u8, u8, u8, u8) = (255, 255, 255, 255);
const BUTTON_MARGIN: i32 = 20;
const BUTTON_H: u32 = 36;
const BUTTON_GAP: i32 = 12;
// Room for a section-title label (drawn separately in `draw_menu`) plus
// spacing above the next row of buttons.
const SECTION_GAP: i32 = 46;

/// One clickable rectangle, positioned identically whether it's being
/// drawn (`draw_menu`) or hit-tested against a click (`on_mouse_down`) —
/// both read from `menu_buttons`, so the two can never drift apart the
/// way a hand-duplicated layout could.
struct ButtonLayout {
    id: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    label: &'static str,
}

impl ButtonLayout {
    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x as f32
            && x < (self.x + self.w as i32) as f32
            && y >= self.y as f32
            && y < (self.y + self.h as i32) as f32
    }
}

/// Every button `menu` currently shows, top to bottom — empty for
/// `Closed`. Pure layout math, no side effects, so both `draw_menu` and
/// `on_mouse_down` can call it freely every tick/click.
fn menu_buttons(menu: MenuState) -> Vec<ButtonLayout> {
    let content_x = PANEL_X + BUTTON_MARGIN;
    let content_w = (PANEL_W as i32 - 2 * BUTTON_MARGIN) as u32;
    let mut buttons = Vec::new();
    match menu {
        MenuState::Closed => {}
        MenuState::Main => {
            let row_y = PANEL_Y + 70;
            buttons.push(ButtonLayout {
                id: BUTTON_SETTINGS,
                x: content_x,
                y: row_y,
                w: content_w,
                h: BUTTON_H,
                label: "Settings",
            });
            buttons.push(ButtonLayout {
                id: BUTTON_EXIT,
                x: content_x,
                y: row_y + BUTTON_H as i32 + BUTTON_GAP,
                w: content_w,
                h: BUTTON_H,
                label: "Exit",
            });
        }
        MenuState::Settings => {
            let preset_w = ((content_w as i32 - 2 * BUTTON_GAP) / 3) as u32;
            let big_row_y = PANEL_Y + 70;
            for (index, &(name, _)) in BIG_BOX_PRESETS.iter().enumerate() {
                buttons.push(ButtonLayout {
                    id: BUTTON_BIG_BOX_PRESET_BASE + index as u32,
                    x: content_x + index as i32 * (preset_w as i32 + BUTTON_GAP),
                    y: big_row_y,
                    w: preset_w,
                    h: BUTTON_H,
                    label: name,
                });
            }
            let small_row_y = big_row_y + BUTTON_H as i32 + SECTION_GAP;
            for (index, &(name, _)) in SMALL_BOX_PRESETS.iter().enumerate() {
                buttons.push(ButtonLayout {
                    id: BUTTON_SMALL_BOX_PRESET_BASE + index as u32,
                    x: content_x + index as i32 * (preset_w as i32 + BUTTON_GAP),
                    y: small_row_y,
                    w: preset_w,
                    h: BUTTON_H,
                    label: name,
                });
            }
            let back_y = small_row_y + BUTTON_H as i32 + SECTION_GAP;
            buttons.push(ButtonLayout {
                id: BUTTON_BACK,
                x: content_x,
                y: back_y,
                w: content_w,
                h: BUTTON_H,
                label: "Back",
            });
        }
    }
    buttons
}

/// Draws the pause menu/settings panel — a no-op while `Closed`. A solid
/// backdrop panel (not a full-screen dim: this renderer's `DrawRect`
/// doesn't blend alpha for filled rects yet, so a translucent overlay
/// would just render opaque) with a title, section labels for `Settings`,
/// and every button from `menu_buttons`.
fn draw_menu(menu: MenuState) {
    if menu == MenuState::Closed {
        return;
    }
    publish(
        DrawRect::TOPIC,
        &DrawRect {
            x: PANEL_X,
            y: PANEL_Y,
            w: PANEL_W,
            h: PANEL_H,
            filled: true,
            color: PANEL_BG_COLOR,
            layer: MENU_LAYER,
            screen_space: true,
        }
        .encode(),
    );
    publish(
        DrawRect::TOPIC,
        &DrawRect {
            x: PANEL_X,
            y: PANEL_Y,
            w: PANEL_W,
            h: PANEL_H,
            filled: false,
            color: PANEL_BORDER_COLOR,
            layer: MENU_LAYER,
            screen_space: true,
        }
        .encode(),
    );

    let title = match menu {
        MenuState::Main => "Paused (Esc to resume)",
        MenuState::Settings => "Settings",
        MenuState::Closed => "",
    };
    publish(
        DrawText::TOPIC,
        &DrawText {
            text: title,
            x: PANEL_X + BUTTON_MARGIN,
            y: PANEL_Y + 20,
            size: 18,
            color: PANEL_TITLE_COLOR,
            layer: MENU_LAYER,
            screen_space: true,
        }
        .encode(),
    );

    if menu == MenuState::Settings {
        publish(
            DrawText::TOPIC,
            &DrawText {
                text: "Big box color",
                x: PANEL_X + BUTTON_MARGIN,
                y: PANEL_Y + 52,
                size: 14,
                color: SECTION_LABEL_COLOR,
                layer: MENU_LAYER,
                screen_space: true,
            }
            .encode(),
        );
        let small_label_y = PANEL_Y + 70 + BUTTON_H as i32 + 10;
        publish(
            DrawText::TOPIC,
            &DrawText {
                text: "Small box color",
                x: PANEL_X + BUTTON_MARGIN,
                y: small_label_y,
                size: 14,
                color: SECTION_LABEL_COLOR,
                layer: MENU_LAYER,
                screen_space: true,
            }
            .encode(),
        );
    }

    for button in menu_buttons(menu) {
        publish(
            DrawRect::TOPIC,
            &DrawRect {
                x: button.x,
                y: button.y,
                w: button.w,
                h: button.h,
                filled: true,
                color: BUTTON_COLOR,
                layer: MENU_LAYER,
                screen_space: true,
            }
            .encode(),
        );
        // Rough centering assuming ~8px average glyph width at size 16 —
        // good enough for this demo's short labels, not measured text.
        let text_x = button.x + (button.w as i32 - button.label.len() as i32 * 8) / 2;
        let text_y = button.y + (button.h as i32 - 16) / 2;
        publish(
            DrawText::TOPIC,
            &DrawText {
                text: button.label,
                x: text_x,
                y: text_y,
                size: 16,
                color: BUTTON_TEXT_COLOR,
                layer: MENU_LAYER,
                screen_space: true,
            }
            .encode(),
        );
    }
}

/// Left-click hit-testing against whatever `menu_buttons` the current menu
/// shows — a no-op while `Closed` (nothing to click) or for any click that
/// doesn't land inside a button.
fn on_mouse_down(x: f32, y: f32) {
    let menu = STATE.with(|state| state.borrow().menu);
    if menu == MenuState::Closed {
        return;
    }
    for button in menu_buttons(menu) {
        if button.contains(x, y) {
            on_button_clicked(button.id);
            break;
        }
    }
}

/// Esc toggles `Closed` <-> whatever the menu currently is — `Settings`
/// has its own Back button to step down to `Main` instead, so Esc only
/// ever fully opens or fully closes. The only place `menu` transitions
/// to/from `Closed`, so it's the only place that needs to publish
/// `SetPaused` — every other menu transition (`Main` <-> `Settings`)
/// leaves the game already paused. Pausing here (not just zeroing the
/// robot's own commanded velocity, which on_tick already does) stops
/// every entity, including one still settling from a recent push, on
/// exactly the frame Esc was pressed instead of letting it keep drifting
/// in the background while the menu is up.
fn toggle_menu() {
    let paused = STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.menu = if state.menu == MenuState::Closed {
            MenuState::Main
        } else {
            MenuState::Closed
        };
        state.menu != MenuState::Closed
    });
    publish_entity_op(EntityOp::SetPaused { paused });
}

fn on_button_clicked(id: u32) {
    if id == BUTTON_SETTINGS {
        STATE.with(|state| state.borrow_mut().menu = MenuState::Settings);
    } else if id == BUTTON_EXIT {
        request_exit();
    } else if id == BUTTON_BACK {
        STATE.with(|state| state.borrow_mut().menu = MenuState::Main);
    } else if let Some(color) = preset_color(id, BUTTON_BIG_BOX_PRESET_BASE, &BIG_BOX_PRESETS) {
        STATE.with(|state| state.borrow_mut().big_box_color = color);
        for entity_id in BIG_BOX_IDS {
            publish_entity_op(EntityOp::SetColor { entity_id, color });
        }
    } else if let Some(color) = preset_color(id, BUTTON_SMALL_BOX_PRESET_BASE, &SMALL_BOX_PRESETS) {
        STATE.with(|state| state.borrow_mut().small_box_color = color);
        for entity_id in SMALL_BOX_IDS {
            publish_entity_op(EntityOp::SetColor { entity_id, color });
        }
    }
}

struct Component;

impl Guest for Component {
    fn init() {
        subscribe(KeyDown::TOPIC);
        subscribe(KeyUp::TOPIC);
        subscribe(GamepadAxis::TOPIC);
        subscribe(Collision::TOPIC);
        subscribe(MouseDown::TOPIC);
        subscribe("core/tick");

        let load_sprite = LoadSprite {
            id: SPRITE_ID,
            png_bytes: SPRITE_PNG,
        };
        publish(LoadSprite::TOPIC, &load_sprite.encode());

        // Tile placement (grass outside the wall, bricks inside) lives in
        // level.tmx's own "Ground" layer, real Tiled data — game-core
        // parses and renders it (see its own doc comment on load_tilemap)
        // via the `tiled` crate, matching each embedded `<tileset name=...>`
        // to the image bytes supplied here by name.
        let load_tilemap = LoadTilemap {
            tmx_bytes: LEVEL_TMX,
            tileset_images: vec![
                TilesetImage {
                    name: GRASS_TILESET_NAME,
                    sprite_id: GRASS_SPRITE_ID,
                    png_bytes: TILESET_GRASS_PNG,
                },
                TilesetImage {
                    name: BRICK_TILESET_NAME,
                    sprite_id: BRICK_SPRITE_ID,
                    png_bytes: TILESET_BRICKS_PNG,
                },
            ],
        };
        publish(LoadTilemap::TOPIC, &load_tilemap.encode());

        // The level is now much bigger than the window (level.tmx is
        // 120x90 tiles), so the camera needs to actually follow the
        // controlled entity rather than sit fixed at the origin — clamped
        // to the level's bounds (game-core's own EntityOp::SetCameraFollow
        // doc comment) so it stops panning once an edge would come into
        // view, instead of showing past the map.
        publish_entity_op(EntityOp::SetCameraFollow {
            entity_id: CONTROLLED_ENTITY_ID,
            viewport_w: SCREEN_WIDTH as f32,
            viewport_h: SCREEN_HEIGHT as f32,
        });

        // ~82ms, 110Hz — a low thud for footsteps.
        let footstep = synthesize_tone(110.0, 9, 0.5);
        publish(
            LoadSound::TOPIC,
            &LoadSound {
                id: FOOTSTEP_SOUND_ID,
                bytes: &footstep,
            }
            .encode(),
        );
        // ~120ms, 300Hz — a higher blip for a hit, distinct from footsteps.
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
            x: LEVEL_ORIGIN_X + 60.0,
            y: LEVEL_ORIGIN_Y + 60.0,
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
        spawn_big_box(2, LEVEL_ORIGIN_X + 350.0, LEVEL_ORIGIN_Y + 60.0);
        spawn_big_box(3, LEVEL_ORIGIN_X + 350.0, LEVEL_ORIGIN_Y + 160.0);
        spawn_big_box(4, LEVEL_ORIGIN_X + 350.0, LEVEL_ORIGIN_Y + 260.0);
        spawn_big_box(5, LEVEL_ORIGIN_X + 120.0, LEVEL_ORIGIN_Y + 260.0);

        // Two blue Frictionless squares: pushable, but carry no momentum —
        // pushing the robot into one moves it, but it stops the instant
        // contact ends, unlike the purple Dynamic big boxes above.
        spawn_small_box(6, LEVEL_ORIGIN_X + 220.0, LEVEL_ORIGIN_Y + 60.0);
        spawn_small_box(7, LEVEL_ORIGIN_X + 220.0, LEVEL_ORIGIN_Y + 260.0);

        // Two stationary red hazard triangles — the robot loses life on
        // contact (see on_message's Collision handling). Positioned clear
        // of every other spawn above.
        spawn_hazard(8, LEVEL_ORIGIN_X + 60.0, LEVEL_ORIGIN_Y + 160.0);
        spawn_hazard(9, LEVEL_ORIGIN_X + 400.0, LEVEL_ORIGIN_Y + 200.0);

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.life = FULL_LIFE;
            state.big_box_color = BIG_BOX_COLOR;
            state.small_box_color = SMALL_BOX_COLOR;
        });

        log(
            Level::Info,
            "init: tilemap + sprite loaded; WASD/gamepad-left-stick moves the entity into big/small boxes and hazards; H toggles hitbox outlines, Esc opens the menu",
        );
    }

    fn on_tick(dt: f32) {
        // Held input is ignored entirely while the menu is open — the
        // robot stops rather than keeps moving under the last commanded
        // velocity while the player is looking at a menu.
        let (vx, vy) = STATE.with(|state| {
            let state = state.borrow();
            if state.menu == MenuState::Closed {
                state.held.velocity()
            } else {
                (0.0, 0.0)
            }
        });
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
            // timer has expired back to its normal color. Reads the *live*
            // big-box color (a Settings preset click may have changed it
            // since the flash started), not the original BIG_BOX_COLOR
            // constant.
            let big_box_color = state.big_box_color;
            state.flashing.retain(|&entity_id, remaining| {
                *remaining -= dt;
                if *remaining > 0.0 {
                    true
                } else {
                    publish_entity_op(EntityOp::SetColor {
                        entity_id,
                        color: revert_color(entity_id, big_box_color),
                    });
                    false
                }
            });
        });

        let (score, life, menu) = STATE.with(|state| {
            let state = state.borrow();
            (state.score, state.life, state.menu)
        });
        draw_hud(score, life);
        draw_menu(menu);
    }

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        match topic.as_str() {
            KeyDown::TOPIC => {
                if let Ok(message) = KeyDown::decode(&payload) {
                    match message.key {
                        "H" => toggle_debug_hitboxes(),
                        "Escape" => toggle_menu(),
                        _ => set_key_held(message.key, true),
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
            MouseDown::TOPIC => {
                if let Ok(message) = MouseDown::decode(&payload) {
                    // 1: SDL's left mouse button code (see MouseDown's own
                    // doc comment) — the only button this demo's menu reacts
                    // to.
                    if message.button == 1 {
                        on_mouse_down(message.x, message.y);
                    }
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
/// `on_collision` ever flashes). `big_box_color` is the *current* live
/// color (a Settings preset click may have changed it since the flash
/// started), not necessarily the original `BIG_BOX_COLOR` constant.
fn revert_color(entity_id: u32, big_box_color: (u8, u8, u8, u8)) -> (u8, u8, u8, u8) {
    if is_hazard(entity_id) {
        HAZARD_COLOR
    } else {
        big_box_color
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
