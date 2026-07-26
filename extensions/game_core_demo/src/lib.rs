wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});

use std::cell::RefCell;
use std::collections::HashMap;

use bones::core::host_api::{
    list_display_modes, log, native_display_mode, publish, request_exit, subscribe, DisplayMode,
    Level,
};
use bones_messages::audio::{LoadSound, PlaySound};
use bones_messages::game_core::{
    BodyKind, Collision, EntityOp, EntityOpMessage, LoadTilemap, PhysicsWorlds, Shape, Sprite,
    TilesetImage,
};
use bones_messages::gfx::{DrawRect, DrawText, LoadSprite, SetDisplay, TextAlign};
use bones_messages::input::{GamepadAxis, KeyDown, KeyUp, MouseDown};
use bones_messages::renderer::DisplayChanged;
use bones_messages::{DecodeMessage, EncodeMessage, Message};
use game_ui::{ButtonLayout, Canvas, DrawCommand, MenuLayout, Rect};

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
// extension only supplies the tileset image and the sprite id to
// register it under, matched to the .tmx's embedded `<tileset name=...>`
// by name. One image supplies both the grass margin and the interior's
// rock paving (level.tmx's own generator script documents where each
// lives within it) - no separate bricks tileset since increment 15.
const TILESET_GRASS_PNG: &[u8] = include_bytes!("assets/tileset_grass.png");
const GRASS_SPRITE_ID: u32 = 2;
const GRASS_TILESET_NAME: &str = "grass";

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
// Entity id ranges, each starting right after the previous - scatter_grid's
// own loop assigns ids sequentially within whichever range matches the
// type it picked for a given cell, so these three counts are the only
// place the total (48) is stated.
const NUM_BIG_BOXES: u32 = 32;
const NUM_SMALL_BOXES: u32 = 8;
const NUM_HAZARDS: u32 = 8;
const BIG_BOX_ID_START: u32 = 2;
const SMALL_BOX_ID_START: u32 = BIG_BOX_ID_START + NUM_BIG_BOXES;
const HAZARD_ID_START: u32 = SMALL_BOX_ID_START + NUM_SMALL_BOXES;
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
const FOOTSTEP_INTERVAL_SECONDS: f32 = 0.35;

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
const BUTTON_FULLSCREEN_TOGGLE: u32 = 4;
const BUTTON_RESOLUTION_TOGGLE: u32 = 5;
// Presets/options start well clear of the fixed ids above; each range is
// spaced far enough apart (10 ids each) that none could ever grow into a
// neighboring one unnoticed - `MAX_RESOLUTION_OPTIONS` (below) comfortably
// fits under 10.
const BUTTON_BIG_BOX_PRESET_BASE: u32 = 10;
const BUTTON_SMALL_BOX_PRESET_BASE: u32 = 20;
const BUTTON_ZOOM_PRESET_BASE: u32 = 30;
const BUTTON_RESOLUTION_OPTION_BASE: u32 = 40;

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

// Capped so the Settings panel's expanded height stays the same regardless
// of how many modes a given monitor reports (some report a dozen+) - kept
// comfortably under the 10-id spacing `BUTTON_RESOLUTION_OPTION_BASE` gets.
const MAX_RESOLUTION_OPTIONS: usize = 6;

/// This session's resolution option list: every mode `list_display_modes`
/// reports, deduped/sorted ascending, with `native_display_mode` folded in
/// (so "the max" is always present even if the fullscreen-mode query omits
/// it on some platform) — falls back to 3 safe hardcoded sizes if the query
/// itself came back empty (e.g. no display attached). Stride-sampled down
/// to `MAX_RESOLUTION_OPTIONS` if longer, always keeping the smallest and
/// largest entries. `gfx::SetDisplay`'s own doc comment: the renderer
/// scales world content and screen_space UI independently to fit whatever
/// size is actually applied, so picking any of these has no ripple effect
/// on game-core's camera math or this demo's HUD layout.
fn resolution_options() -> Vec<(u32, u32)> {
    let mut modes: Vec<(u32, u32)> = list_display_modes()
        .into_iter()
        .map(|DisplayMode { width, height }| (width, height))
        .collect();
    if let Some(DisplayMode { width, height }) = native_display_mode() {
        modes.push((width, height));
    }
    modes.sort_unstable();
    modes.dedup();
    if modes.is_empty() {
        modes = vec![(800, 600), (1024, 768), (1280, 720)];
    }
    if modes.len() <= MAX_RESOLUTION_OPTIONS {
        return modes;
    }
    let last = modes.len() - 1;
    let stride = last as f32 / (MAX_RESOLUTION_OPTIONS - 1) as f32;
    (0..MAX_RESOLUTION_OPTIONS)
        .map(|index| modes[((index as f32 * stride).round() as usize).min(last)])
        .collect()
}

type ZoomPreset = (&'static str, f32);

const ZOOM_PRESETS: [ZoomPreset; 3] = [("1.0x", 1.0), ("1.5x", 1.5), ("2.0x", 2.0)];

/// `id`'s preset zoom — same addressing convention as `preset_color`.
fn preset_zoom(id: u32) -> Option<f32> {
    let index = id.checked_sub(BUTTON_ZOOM_PRESET_BASE)? as usize;
    ZOOM_PRESETS.get(index).map(|&(_, zoom)| zoom)
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
    // Live display settings, changed by Settings/Display clicks and
    // republished to the engine (gfx::SetDisplay, EntityOp::SetCameraFollow)
    // on every change. `resolution` starts at (SCREEN_WIDTH, SCREEN_HEIGHT)
    // (see init) — not derivable from Default, same reasoning as the box
    // colors above — but is also kept in sync reactively: on_message
    // updates it from every renderer/display-changed event, since
    // fullscreen may not honor whatever was last requested.
    resolution: (u32, u32),
    fullscreen: bool,
    zoom: f32,
    // Queried once via resolution_options() in init and never changed
    // after - the Settings/Display resolution dropdown's fixed option list.
    resolution_options: Vec<(u32, u32)>,
    // Whether that dropdown is currently showing its option list.
    resolution_expanded: bool,
    // `Some` freezes gameplay (EntityOp::SetPaused published alongside, in
    // on_collision) and switches on_tick to drawing the end screen instead
    // of the normal HUD/menu - `None` the entire time gameplay is ongoing.
    // Enter (KeyDown) is the only way back to `None`, via restart().
    game_over: Option<Outcome>,
}

/// Which end screen `State::game_over` is currently showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    GameOver,
    Win,
}

const FULL_LIFE: u32 = 3;
// Reachable now that a scattered layout (increment 22) fills the arena with
// many more big boxes than the original handful clustered near spawn.
const WIN_SCORE: u32 = 100;

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

// Interior play area (level.tmx's own generator script): world x:[96,1824),
// y:[96,1344) - orig-space (LEVEL_ORIGIN-relative, since that offset cancels
// out of both ends the same way it does for every spawn coordinate here)
// that's x:[32,1760), y:[32,1280). `SCATTER_MIN`/`SCATTER_MAX_*` inset that
// by a further 120px margin so nothing spawns flush against a wall or
// overlapping the robot's own spawn point (60, 60) - comfortably clear at
// the grid's own nearest cell, (152, 152).
const SCATTER_MIN: f32 = 152.0;
const SCATTER_MAX_X: f32 = 1640.0;
const SCATTER_MAX_Y: f32 = 1160.0;
// 8x6 = 48, matching NUM_BIG_BOXES + NUM_SMALL_BOXES + NUM_HAZARDS exactly
// - scatter_grid's loop has one cell per entity, no gaps needed for
// "not too dense": at this column/row count the ~210x200px cell spacing
// already leaves generous room around each ~40-48px box/hazard.
const GRID_COLUMNS: u32 = 8;
const GRID_ROWS: u32 = 6;

/// `index`'s (x, y) in the scatter grid (row-major, `GRID_COLUMNS` wide,
/// spanning the full `SCATTER_MIN..SCATTER_MAX_*` usable area), plus a
/// small deterministic per-index jitter so the layout doesn't read as an
/// obviously rigid grid — no `rand` dependency, so the same layout every
/// run (restart() included), which is a feature: reproducible for testing,
/// not a limitation.
fn scatter_position(index: u32) -> (f32, f32) {
    let column = (index % GRID_COLUMNS) as f32;
    let row = (index / GRID_COLUMNS) as f32;
    let column_spacing = (SCATTER_MAX_X - SCATTER_MIN) / (GRID_COLUMNS - 1) as f32;
    let row_spacing = (SCATTER_MAX_Y - SCATTER_MIN) / (GRID_ROWS - 1) as f32;
    let jitter_x = (index.wrapping_mul(37) % 41) as f32 - 20.0;
    let jitter_y = (index.wrapping_mul(53) % 41) as f32 - 20.0;
    (
        LEVEL_ORIGIN_X + SCATTER_MIN + column * column_spacing + jitter_x,
        LEVEL_ORIGIN_Y + SCATTER_MIN + row * row_spacing + jitter_y,
    )
}

/// Spawns the controlled entity plus every big box/small box/hazard at
/// their starting positions. Called once from `init`, and again from
/// `restart` — `EntityOp::Spawn` on an `entity_id` already in use replaces
/// that entity (the same semantics `gfx::DrawSprite` batches already use),
/// so a restart returns every entity to exactly where it began, not
/// wherever play left it.
fn spawn_entities() {
    // The controlled entity: driven by set-velocity from on_tick, below —
    // the only entity in this demo that uses the robot sprite. Its
    // collider is narrower than the sprite frame (CONTROLLED_HALF_EXTENT
    // vs. FRAME_SIZE/2.0) — the robot's actual body width, not its full
    // drawn frame including empty margin either side. Registered in both
    // physics worlds at once (ADR-021, `PhysicsWorlds::BOTH`) — this
    // demo's example of a single entity genuinely simulated by two
    // independent backends simultaneously; `retro` outranks `rapier2d` in
    // `PhysicsWorldKind::PRIORITY`, so the robot's drawn position tracks
    // the no-mass, no-solver world while its rapier2d copy (still pushed
    // by/pushing big boxes) is snapped to match every tick.
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

    // Scattered across the whole interior (scatter_position), one cell per
    // entity: mostly stationary purple big-box squares — the controlled
    // entity (and, if pushed into a neighbor, one box into another)
    // visibly stops against each one, proving entity-entity collision
    // alongside the tilemap's entity-terrain collision, and the robot
    // hitting one scores a point (see on_collision) - with WIN_SCORE (100)
    // reachable by re-hitting boxes already pushed clear, not just needing
    // 100 distinct ones. Every 6th cell is a stationary red hazard
    // triangle instead (life -= 1 on contact); every remaining 3rd is a
    // blue Frictionless small box (pushable, no momentum, unlike the big
    // boxes) rather than another big box - real variety, not a uniform
    // field of identical squares.
    let mut next_big_box_id = BIG_BOX_ID_START;
    let mut next_small_box_id = SMALL_BOX_ID_START;
    let mut next_hazard_id = HAZARD_ID_START;
    for index in 0..(NUM_BIG_BOXES + NUM_SMALL_BOXES + NUM_HAZARDS) {
        let (x, y) = scatter_position(index);
        if index % 6 == 0 {
            spawn_hazard(next_hazard_id, x, y);
            next_hazard_id += 1;
        } else if index % 3 == 0 {
            spawn_small_box(next_small_box_id, x, y);
            next_small_box_id += 1;
        } else {
            spawn_big_box(next_big_box_id, x, y);
            next_big_box_id += 1;
        }
    }
}

/// Resets gameplay to a fresh start: score/life/every entity back to their
/// initial state (`spawn_entities` again, replacing whatever play left
/// them at), closes any open menu, and unpauses — the only way back to
/// `None` from `State::game_over`.
fn restart() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.score = 0;
        state.life = FULL_LIFE;
        state.big_box_color = BIG_BOX_COLOR;
        state.small_box_color = SMALL_BOX_COLOR;
        state.game_over = None;
        state.menu = MenuState::Closed;
    });
    spawn_entities();
    publish_entity_op(EntityOp::SetPaused { paused: false });
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
            align: TextAlign::Left,
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
            align: TextAlign::Left,
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
const PANEL_X: i32 = (SCREEN_WIDTH - PANEL_W as i32) / 2;
// Fits the title, both color sections, the Display section's fullscreen
// toggle and zoom presets, and the resolution dropdown collapsed to its one
// toggle button - the common case. Expanding the dropdown adds one row per
// option on top of this, so the panel grows (and re-centers) only then;
// `panel_geometry` is the one place both `draw_menu` and `menu_layout`
// compute this from, so they can never drift apart.
const PANEL_H_COLLAPSED: u32 = 480;

/// `(PANEL_Y, PANEL_H)` for the current Settings panel content — vertically
/// re-centered around `SCREEN_HEIGHT` so the extra rows an expanded
/// resolution dropdown adds grow the panel symmetrically instead of just
/// pushing its bottom edge further down.
fn panel_geometry(resolution_expanded: bool, num_resolution_options: usize) -> (i32, u32) {
    let extra = if resolution_expanded {
        // 3 per row (`menu_layout`'s own grid) - rows, not raw option count.
        let rows = num_resolution_options.div_ceil(3) as u32;
        rows * (BUTTON_H + BUTTON_GAP as u32)
    } else {
        0
    };
    let height = PANEL_H_COLLAPSED + extra;
    let y = (SCREEN_HEIGHT - height as i32) / 2;
    (y, height)
}
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

/// Every button `menu` currently shows, top to bottom — empty for
/// `Closed`. Pure layout math (`fullscreen`/`resolution` only change label
/// text, not positions; `resolution_expanded` does change what's laid out
/// below it), no side effects, so both `draw_menu` and `on_mouse_down` can
/// call it freely every tick/click.
fn menu_layout(
    menu: MenuState,
    fullscreen: bool,
    resolution: (u32, u32),
    resolution_expanded: bool,
    resolution_options: &[(u32, u32)],
) -> MenuLayout {
    let content_x = PANEL_X + BUTTON_MARGIN;
    let content_w = (PANEL_W as i32 - 2 * BUTTON_MARGIN) as u32;
    let (panel_y, panel_height) = panel_geometry(resolution_expanded, resolution_options.len());
    let mut buttons = Vec::new();
    match menu {
        MenuState::Closed => {}
        MenuState::Main => {
            let row_y = panel_y + 70;
            buttons.push(ButtonLayout {
                id: BUTTON_SETTINGS,
                bounds: Rect {
                    x: content_x,
                    y: row_y,
                    width: content_w,
                    height: BUTTON_H,
                },
                label: "Settings".to_owned(),
            });
            buttons.push(ButtonLayout {
                id: BUTTON_EXIT,
                bounds: Rect {
                    x: content_x,
                    y: row_y + BUTTON_H as i32 + BUTTON_GAP,
                    width: content_w,
                    height: BUTTON_H,
                },
                label: "Exit".to_owned(),
            });
        }
        MenuState::Settings => {
            let preset_w = ((content_w as i32 - 2 * BUTTON_GAP) / 3) as u32;
            let big_row_y = panel_y + 70;
            for (index, &(name, _)) in BIG_BOX_PRESETS.iter().enumerate() {
                buttons.push(ButtonLayout {
                    id: BUTTON_BIG_BOX_PRESET_BASE + index as u32,
                    bounds: Rect {
                        x: content_x + index as i32 * (preset_w as i32 + BUTTON_GAP),
                        y: big_row_y,
                        width: preset_w,
                        height: BUTTON_H,
                    },
                    label: name.to_owned(),
                });
            }
            let small_row_y = big_row_y + BUTTON_H as i32 + SECTION_GAP;
            for (index, &(name, _)) in SMALL_BOX_PRESETS.iter().enumerate() {
                buttons.push(ButtonLayout {
                    id: BUTTON_SMALL_BOX_PRESET_BASE + index as u32,
                    bounds: Rect {
                        x: content_x + index as i32 * (preset_w as i32 + BUTTON_GAP),
                        y: small_row_y,
                        width: preset_w,
                        height: BUTTON_H,
                    },
                    label: name.to_owned(),
                });
            }
            let fullscreen_row_y = small_row_y + BUTTON_H as i32 + SECTION_GAP;
            buttons.push(ButtonLayout {
                id: BUTTON_FULLSCREEN_TOGGLE,
                bounds: Rect {
                    x: content_x,
                    y: fullscreen_row_y,
                    width: content_w,
                    height: BUTTON_H,
                },
                label: if fullscreen {
                    "Fullscreen: On".to_owned()
                } else {
                    "Fullscreen: Off".to_owned()
                },
            });
            let zoom_row_y = fullscreen_row_y + BUTTON_H as i32 + BUTTON_GAP;
            for (index, &(name, _)) in ZOOM_PRESETS.iter().enumerate() {
                buttons.push(ButtonLayout {
                    id: BUTTON_ZOOM_PRESET_BASE + index as u32,
                    bounds: Rect {
                        x: content_x + index as i32 * (preset_w as i32 + BUTTON_GAP),
                        y: zoom_row_y,
                        width: preset_w,
                        height: BUTTON_H,
                    },
                    label: name.to_owned(),
                });
            }
            let resolution_toggle_row_y = zoom_row_y + BUTTON_H as i32 + SECTION_GAP;
            buttons.push(ButtonLayout {
                id: BUTTON_RESOLUTION_TOGGLE,
                bounds: Rect {
                    x: content_x,
                    y: resolution_toggle_row_y,
                    width: content_w,
                    height: BUTTON_H,
                },
                label: format!(
                    "Resolution: {}x{} {}",
                    resolution.0,
                    resolution.1,
                    if resolution_expanded {
                        "\u{25B4}"
                    } else {
                        "\u{25BE}"
                    }
                ),
            });
            let mut after_resolution_y = resolution_toggle_row_y + BUTTON_H as i32 + BUTTON_GAP;
            if resolution_expanded {
                // 3 per row (same grid the color/zoom presets above use),
                // not one per row - a one-per-row list of up to
                // MAX_RESOLUTION_OPTIONS would make the panel taller than
                // SCREEN_HEIGHT, pushing Back (and the bottom option rows)
                // off the fixed logical canvas entirely.
                for (index, &(width, height)) in resolution_options.iter().enumerate() {
                    let column = index % 3;
                    if column == 0 && index > 0 {
                        after_resolution_y += BUTTON_H as i32 + BUTTON_GAP;
                    }
                    buttons.push(ButtonLayout {
                        id: BUTTON_RESOLUTION_OPTION_BASE + index as u32,
                        bounds: Rect {
                            x: content_x + column as i32 * (preset_w as i32 + BUTTON_GAP),
                            y: after_resolution_y,
                            width: preset_w,
                            height: BUTTON_H,
                        },
                        label: format!("{width}x{height}"),
                    });
                }
                after_resolution_y += BUTTON_H as i32 + BUTTON_GAP;
            }
            let back_y = after_resolution_y - BUTTON_GAP + SECTION_GAP;
            buttons.push(ButtonLayout {
                id: BUTTON_BACK,
                bounds: Rect {
                    x: content_x,
                    y: back_y,
                    width: content_w,
                    height: BUTTON_H,
                },
                label: "Back".to_owned(),
            });
        }
    }
    MenuLayout {
        panel: Rect {
            x: PANEL_X,
            y: panel_y,
            width: PANEL_W,
            height: panel_height,
        },
        buttons,
    }
}

/// Draws the pause menu/settings panel — a no-op while `Closed`. A solid
/// backdrop panel (not a full-screen dim: this renderer's `DrawRect`
/// doesn't blend alpha for filled rects yet, so a translucent overlay
/// would just render opaque) with a title, section labels for `Settings`,
/// and every button from `menu_layout`.
fn draw_menu(
    menu: MenuState,
    fullscreen: bool,
    resolution: (u32, u32),
    resolution_expanded: bool,
    resolution_options: &[(u32, u32)],
) {
    if menu == MenuState::Closed {
        return;
    }
    let layout = menu_layout(
        menu,
        fullscreen,
        resolution,
        resolution_expanded,
        resolution_options,
    );
    let panel_y = layout.panel.y;
    DrawCommand::rectangle(layout.panel, true, PANEL_BG_COLOR, MENU_LAYER).publish_with(publish);
    DrawCommand::rectangle(layout.panel, false, PANEL_BORDER_COLOR, MENU_LAYER)
        .publish_with(publish);

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
            y: panel_y + 20,
            size: 18,
            color: PANEL_TITLE_COLOR,
            layer: MENU_LAYER,
            screen_space: true,
            align: TextAlign::Left,
        }
        .encode(),
    );

    if menu == MenuState::Settings {
        publish(
            DrawText::TOPIC,
            &DrawText {
                text: "Big box color",
                x: PANEL_X + BUTTON_MARGIN,
                y: panel_y + 52,
                size: 14,
                color: SECTION_LABEL_COLOR,
                layer: MENU_LAYER,
                screen_space: true,
                align: TextAlign::Left,
            }
            .encode(),
        );
        let small_row_y = panel_y + 70 + BUTTON_H as i32 + SECTION_GAP;
        let small_label_y = panel_y + 70 + BUTTON_H as i32 + 10;
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
                align: TextAlign::Left,
            }
            .encode(),
        );
        let display_label_y = small_row_y + BUTTON_H as i32 + 10;
        publish(
            DrawText::TOPIC,
            &DrawText {
                text: "Display",
                x: PANEL_X + BUTTON_MARGIN,
                y: display_label_y,
                size: 14,
                color: SECTION_LABEL_COLOR,
                layer: MENU_LAYER,
                screen_space: true,
                align: TextAlign::Left,
            }
            .encode(),
        );
    }

    for button in layout.buttons {
        DrawCommand::rectangle(button.bounds, true, BUTTON_COLOR, MENU_LAYER).publish_with(publish);
        // Rough centering assuming ~8px average glyph width at size 16 —
        // good enough for this demo's short labels, not measured text.
        let text_x =
            button.bounds.x + (button.bounds.width as i32 - button.label.len() as i32 * 8) / 2;
        let text_y = button.bounds.y + (button.bounds.height as i32 - 16) / 2;
        DrawCommand::text(
            button.label,
            text_x,
            text_y,
            16,
            BUTTON_TEXT_COLOR,
            MENU_LAYER,
        )
        .publish_with(publish);
    }
}

// Above MENU_LAYER, so the end screen always wins if both were somehow
// drawn the same tick (shouldn't happen — toggle_menu is a no-op once
// State::game_over is Some — but layering defensively costs nothing).
const END_LAYER: u8 = 7;
const END_OVERLAY_COLOR: (u8, u8, u8, u8) = (10, 10, 15, 255);
const GAME_OVER_TITLE_COLOR: (u8, u8, u8, u8) = (220, 40, 40, 255);
const WIN_TITLE_COLOR: (u8, u8, u8, u8) = (255, 215, 0, 255);

/// One line of screen_space text, horizontally centered on `SCREEN_WIDTH` —
/// same rough ~half-glyph-width-per-point centering estimate `draw_menu`'s
/// button labels already use, just applied around the screen's own center
/// instead of a button's.
fn draw_centered_text(text: &str, y: i32, size: u16, color: (u8, u8, u8, u8)) {
    let width_estimate = text.len() as i32 * size as i32 / 2;
    let x = (SCREEN_WIDTH - width_estimate) / 2;
    publish(
        DrawText::TOPIC,
        &DrawText {
            text,
            x,
            y,
            size,
            color,
            layer: END_LAYER,
            screen_space: true,
            align: TextAlign::Left,
        }
        .encode(),
    );
}

/// A full-canvas backdrop (opaque — `DrawRect` doesn't blend alpha for
/// filled rects yet, see `draw_menu`'s own doc comment) plus `outcome`'s
/// message, called every tick while `State::game_over` is `Some`. `score`
/// is only shown for `Win` — `GameOver` doesn't need it, life having
/// already hit zero is the whole story.
fn draw_end_screen(outcome: Outcome, score: u32) {
    publish(
        DrawRect::TOPIC,
        &DrawRect {
            x: 0,
            y: 0,
            w: SCREEN_WIDTH as u32,
            h: SCREEN_HEIGHT as u32,
            filled: true,
            color: END_OVERLAY_COLOR,
            layer: END_LAYER,
            screen_space: true,
        }
        .encode(),
    );
    match outcome {
        Outcome::GameOver => {
            draw_centered_text(
                "GAME OVER",
                SCREEN_HEIGHT / 2 - 30,
                48,
                GAME_OVER_TITLE_COLOR,
            );
            draw_centered_text(
                "Press Enter to retry",
                SCREEN_HEIGHT / 2 + 30,
                20,
                BUTTON_TEXT_COLOR,
            );
        }
        Outcome::Win => {
            draw_centered_text("You Win!", SCREEN_HEIGHT / 2 - 60, 48, WIN_TITLE_COLOR);
            let points_text = format!("You got {score} points!");
            draw_centered_text(&points_text, SCREEN_HEIGHT / 2, 22, BUTTON_TEXT_COLOR);
            draw_centered_text(
                "Press Enter to restart",
                SCREEN_HEIGHT / 2 + 40,
                20,
                BUTTON_TEXT_COLOR,
            );
        }
    }
}

/// Left-click hit-testing against whatever `menu_layout` the current menu
/// shows — a no-op while `Closed` (nothing to click) or for any click that
/// doesn't land inside a button.
fn on_mouse_down(x: f32, y: f32) {
    let (menu, fullscreen, resolution, resolution_expanded, resolution_options) =
        STATE.with(|state| {
            let state = state.borrow();
            (
                state.menu,
                state.fullscreen,
                state.resolution,
                state.resolution_expanded,
                state.resolution_options.clone(),
            )
        });
    if menu == MenuState::Closed {
        return;
    }
    // `input/mouse-down` reports physical window pixels (platform has no
    // concept of the renderer's fixed logical/UI space - see
    // gfx::SetDisplay's own doc comment), but every button position from
    // `menu_layout` is in that fixed SCREEN_WIDTH/SCREEN_HEIGHT space, the
    // same one screen_space draws stretch to fill. Converting back by
    // `resolution` keeps hit-testing aligned with what's actually drawn -
    // `resolution` is the renderer's own confirmed actual size
    // (renderer/display-changed, see on_message), not a guess, so this
    // stays correct in fullscreen too.
    let layout = menu_layout(
        menu,
        fullscreen,
        resolution,
        resolution_expanded,
        &resolution_options,
    );
    if let Some((_, id)) = layout.hit_test(
        Canvas::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32),
        x,
        y,
        resolution,
    ) {
        on_button_clicked(id);
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
/// A no-op while `State::game_over` is `Some` — the end screen has its own
/// Enter-to-restart, not the pause menu, and the game is already paused
/// for a different reason (SetPaused published from on_collision, not
/// here) that restart() alone should undo.
fn toggle_menu() {
    let paused = STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.game_over.is_some() {
            return None;
        }
        state.menu = if state.menu == MenuState::Closed {
            MenuState::Main
        } else {
            MenuState::Closed
        };
        Some(state.menu != MenuState::Closed)
    });
    if let Some(paused) = paused {
        publish_entity_op(EntityOp::SetPaused { paused });
    }
}

fn on_button_clicked(id: u32) {
    if id == BUTTON_SETTINGS {
        STATE.with(|state| state.borrow_mut().menu = MenuState::Settings);
    } else if id == BUTTON_EXIT {
        request_exit();
    } else if id == BUTTON_BACK {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.menu = MenuState::Main;
            // So reopening Settings later starts collapsed again, not
            // wherever this visit happened to leave it.
            state.resolution_expanded = false;
        });
    } else if let Some(color) = preset_color(id, BUTTON_BIG_BOX_PRESET_BASE, &BIG_BOX_PRESETS) {
        STATE.with(|state| state.borrow_mut().big_box_color = color);
        for entity_id in BIG_BOX_ID_START..BIG_BOX_ID_START + NUM_BIG_BOXES {
            publish_entity_op(EntityOp::SetColor { entity_id, color });
        }
    } else if let Some(color) = preset_color(id, BUTTON_SMALL_BOX_PRESET_BASE, &SMALL_BOX_PRESETS) {
        STATE.with(|state| state.borrow_mut().small_box_color = color);
        for entity_id in SMALL_BOX_ID_START..SMALL_BOX_ID_START + NUM_SMALL_BOXES {
            publish_entity_op(EntityOp::SetColor { entity_id, color });
        }
    } else if id == BUTTON_FULLSCREEN_TOGGLE {
        let (resolution, fullscreen) = STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.fullscreen = !state.fullscreen;
            (state.resolution, state.fullscreen)
        });
        publish_display(resolution, fullscreen);
    } else if let Some(zoom) = preset_zoom(id) {
        STATE.with(|state| state.borrow_mut().zoom = zoom);
        publish_entity_op(EntityOp::SetCameraFollow {
            entity_id: CONTROLLED_ENTITY_ID,
            viewport_w: SCREEN_WIDTH as f32,
            viewport_h: SCREEN_HEIGHT as f32,
            zoom,
        });
    } else if id == BUTTON_RESOLUTION_TOGGLE {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.resolution_expanded = !state.resolution_expanded;
        });
    } else {
        let selected = STATE.with(|state| {
            let state = state.borrow();
            resolution_option_at(id, &state.resolution_options)
        });
        if let Some(resolution) = selected {
            let fullscreen = STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.resolution = resolution;
                state.resolution_expanded = false;
                state.fullscreen
            });
            publish_display(resolution, fullscreen);
        }
    }
}

/// `id`'s resolution option within `options` — same addressing convention
/// as `preset_color`.
fn resolution_option_at(id: u32, options: &[(u32, u32)]) -> Option<(u32, u32)> {
    let index = id.checked_sub(BUTTON_RESOLUTION_OPTION_BASE)? as usize;
    options.get(index).copied()
}

fn publish_display(resolution: (u32, u32), fullscreen: bool) {
    let (width, height) = resolution;
    publish(
        SetDisplay::TOPIC,
        &SetDisplay {
            width,
            height,
            fullscreen,
        }
        .encode(),
    );
}

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        subscribe(KeyDown::TOPIC);
        subscribe(KeyUp::TOPIC);
        subscribe(GamepadAxis::TOPIC);
        subscribe(Collision::TOPIC);
        subscribe(MouseDown::TOPIC);
        subscribe(DisplayChanged::TOPIC);
        subscribe("core/tick");

        let load_sprite = LoadSprite {
            id: SPRITE_ID,
            png_bytes: SPRITE_PNG,
        };
        publish(LoadSprite::TOPIC, &load_sprite.encode());

        // Tile placement (grass margin, rock-paved interior) lives in
        // level.tmx's own "Ground" layer, real Tiled data — game-core
        // parses and renders it (see its own doc comment on load_tilemap)
        // via the `tiled` crate, matching the embedded `<tileset name=...>`
        // to the image bytes supplied here by name.
        let load_tilemap = LoadTilemap {
            tmx_bytes: LEVEL_TMX,
            tileset_images: vec![TilesetImage {
                name: GRASS_TILESET_NAME,
                sprite_id: GRASS_SPRITE_ID,
                png_bytes: TILESET_GRASS_PNG,
            }],
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
            zoom: 1.0,
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

        spawn_entities();

        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.life = FULL_LIFE;
            state.big_box_color = BIG_BOX_COLOR;
            state.small_box_color = SMALL_BOX_COLOR;
            state.resolution = (SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32);
            state.fullscreen = false;
            state.zoom = 1.0;
            state.resolution_options = resolution_options();
            state.resolution_expanded = false;
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
            if state.menu == MenuState::Closed && state.game_over.is_none() {
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

        let (
            score,
            life,
            menu,
            fullscreen,
            resolution,
            resolution_expanded,
            resolution_options,
            game_over,
        ) = STATE.with(|state| {
            let state = state.borrow();
            (
                state.score,
                state.life,
                state.menu,
                state.fullscreen,
                state.resolution,
                state.resolution_expanded,
                state.resolution_options.clone(),
                state.game_over,
            )
        });
        draw_hud(score, life);
        draw_menu(
            menu,
            fullscreen,
            resolution,
            resolution_expanded,
            &resolution_options,
        );
        // Drawn last (and covers the whole canvas, opaque) so it's always
        // on top of the HUD/menu above regardless of publish order within
        // this tick - not that menu should ever be open at the same time
        // (toggle_menu is a no-op once game_over is Some).
        if let Some(outcome) = game_over {
            draw_end_screen(outcome, score);
        }
    }

    fn on_message(topic: String, _sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        match topic.as_str() {
            KeyDown::TOPIC => {
                if let Ok(message) = KeyDown::decode(&payload) {
                    match message.key {
                        "H" => toggle_debug_hitboxes(),
                        "Escape" => toggle_menu(),
                        // Only meaningful once State::game_over is Some -
                        // a no-op the rest of the time, not bound to
                        // anything else this demo does with Enter. Both
                        // keycodes: SDL reports the numpad Enter key as
                        // its own distinct "KpEnter", not "Return".
                        "Return" | "KpEnter" => {
                            if STATE.with(|state| state.borrow().game_over.is_some()) {
                                restart();
                            }
                        }
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
            DisplayChanged::TOPIC => {
                // The renderer's confirmation of what the window's actual
                // size ended up being - fullscreen especially may not honor
                // whatever gfx::SetDisplay last requested, so mouse-click
                // hit-testing (on_mouse_down's own rescale) needs the real
                // value, not just this extension's own last guess.
                if let Ok(message) = DisplayChanged::decode(&payload) {
                    STATE.with(|state| {
                        state.borrow_mut().resolution = (message.width, message.height);
                    });
                }
            }
            _ => {}
        }
        None
    }
}

/// Big box ids are `BIG_BOX_ID_START..BIG_BOX_ID_START + NUM_BIG_BOXES`
/// (see `spawn_entities`).
fn is_big_box(entity_id: u32) -> bool {
    (BIG_BOX_ID_START..BIG_BOX_ID_START + NUM_BIG_BOXES).contains(&entity_id)
}

/// Hazard triangle ids are `HAZARD_ID_START..HAZARD_ID_START + NUM_HAZARDS`
/// (see `spawn_entities`).
fn is_hazard(entity_id: u32) -> bool {
    (HAZARD_ID_START..HAZARD_ID_START + NUM_HAZARDS).contains(&entity_id)
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
    // Physics is paused the instant an outcome is set (below), so this
    // shouldn't fire again before restart() — guarded anyway rather than
    // assumed.
    if STATE.with(|state| state.borrow().game_over.is_some()) {
        return;
    }

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
        let outcome = STATE.with(|state| {
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
            let outcome = if state.life == 0 {
                Some(Outcome::GameOver)
            } else if state.score >= WIN_SCORE {
                Some(Outcome::Win)
            } else {
                None
            };
            state.game_over = outcome;
            outcome
        });
        // Freezes every entity (game-core's own tick, ADR: SetPaused skips
        // stepping/collision-publishing entirely) the instant the game
        // ends, the same mechanism the pause menu already uses — nothing
        // keeps moving on the end screen, and no further Collision can
        // fire to re-trigger this.
        if outcome.is_some() {
            publish_entity_op(EntityOp::SetPaused { paused: true });
        }
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
