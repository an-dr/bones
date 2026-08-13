//! Generates the conformance vectors shipped in `wit/vectors/` and checks the
//! committed file still matches what this crate actually encodes.
//!
//! The vectors are what a non-Rust implementation tests against
//! (`wit/wire-format.md`), so the one thing that must never happen is the
//! shipped file describing an encoding the engine no longer produces. Deriving
//! the file from `EncodeMessage` rather than writing it by hand makes that
//! impossible: the bytes in it are, by construction, the bytes on the bus.
//!
//! Regenerate after an intentional encoding change:
//!
//! ```text
//! BONES_WRITE_VECTORS=1 cargo test --test conformance
//! ```
//!
//! That is a deliberate act. An encoding change is an ABI break
//! (`wit/wire-format.md`, ADR-029), so the diff it produces is meant to be
//! read, not waved through.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use bones_messages::audio::{LoadSound, PlayMusic, PlaySound, SetMusicVolume, StopMusic};
use bones_messages::extension_control::{Load, Reload, Unload};
use bones_messages::game_core::{
    BodyKind, Collision, EntityOp, EntityOpMessage, EntityTransform, LoadTilemap, PhysicsWorlds,
    Shape, Sprite, SpritePresentation, TilesetImage,
};
use bones_messages::gfx::{
    Clear, ClearDrawBatch, DrawCircle, DrawLine, DrawRect, DrawSprite, DrawText, DrawTriangle,
    LoadSprite, SetCamera, SetDisplay, TextAlign,
};
use bones_messages::input::{
    GamepadAxis, GamepadButtonDown, GamepadButtonUp, GamepadConnected, GamepadDisconnected,
    KeyDown, KeyUp, MouseDown, MouseMove, MouseUp, MouseWheel,
};
use bones_messages::lifecycle::{Event, LifecycleEvent};
use bones_messages::persistence::Save;
use bones_messages::renderer::{DisplayChanged, LogicalCanvas};
use bones_messages::tick::Tick;
use bones_messages::ui::{Changed, Clicked, Spec, Widget};
use bones_messages::web::{
    ClosePanel, Command as WebCommand, Navigate, OpenPanel, PageMessage, PanelClosed, PanelFailed,
    PanelOpened, PanelSource, SendJson,
};
use bones_messages::window::CloseRequested;
use bones_messages::EncodeMessage;

/// One row of the shipped file: what to encode, and the bytes it becomes.
struct Vector {
    topic: &'static str,
    fields: String,
    hex: String,
}

/// Records a typed message as a vector, taking the topic from the type rather
/// than from a string a refactor could leave stale.
fn vector<M: EncodeMessage>(message: &M, fields: &str) -> Vector {
    Vector {
        topic: M::TOPIC,
        fields: fields.to_string(),
        hex: hex(&message.encode()),
    }
}

/// A direct-send payload, which has an endpoint rather than a topic.
fn send_vector(endpoint: &'static str, payload: Vec<u8>, fields: &str) -> Vector {
    Vector {
        topic: endpoint,
        fields: fields.to_string(),
        hex: hex(&payload),
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(out, "{byte:02x}").expect("writing to a String cannot fail");
    }
    if out.is_empty() {
        out.push('-');
    }
    out
}

fn vectors() -> Vec<Vector> {
    vec![
        // -- core -------------------------------------------------------
        vector(&Tick { dt: 0.5 }, "dt=0.5"),
        vector(
            &LifecycleEvent {
                event: Event::Loaded,
                extension: "hello",
            },
            "event=0;extension=hello",
        ),
        vector(
            &LifecycleEvent {
                event: Event::Stopped,
                extension: "hello",
            },
            "event=4;extension=hello",
        ),
        vector(&Load { extension: "notes" }, "extension=notes"),
        vector(&Unload { extension: "notes" }, "extension=notes"),
        vector(&Reload { extension: "notes" }, "extension=notes"),
        vector(&CloseRequested, "(no fields)"),
        // -- input ------------------------------------------------------
        vector(&KeyDown { key: "A" }, "key=A"),
        vector(&KeyUp { key: "Space" }, "key=Space"),
        vector(
            &MouseDown {
                button: 1,
                x: 12.5,
                y: -3.0,
            },
            "button=1;x=12.5;y=-3",
        ),
        vector(
            &MouseUp {
                button: 3,
                x: 0.0,
                y: 0.0,
            },
            "button=3;x=0;y=0",
        ),
        vector(
            &MouseMove {
                x: 100.0,
                y: 200.0,
                dx: -1.5,
                dy: 2.25,
            },
            "x=100;y=200;dx=-1.5;dy=2.25",
        ),
        vector(&MouseWheel { x: 0.0, y: 1.0 }, "x=0;y=1"),
        vector(&GamepadConnected { id: 7 }, "id=7"),
        vector(&GamepadDisconnected { id: 7 }, "id=7"),
        vector(
            &GamepadButtonDown {
                id: 0,
                button: "south",
            },
            "id=0;button=south",
        ),
        vector(
            &GamepadButtonUp {
                id: 0,
                button: "south",
            },
            "id=0;button=south",
        ),
        vector(
            &GamepadAxis {
                id: 0,
                axis: "leftx",
                value: -1.0,
            },
            "id=0;axis=leftx;value=-1",
        ),
        // -- gfx --------------------------------------------------------
        vector(
            &Clear {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 0xff,
            },
            "r=17;g=34;b=51;a=255",
        ),
        vector(&ClearDrawBatch, "(no fields)"),
        vector(
            &LoadSprite {
                id: 1,
                png_bytes: &[0x89, 0x50, 0x4e, 0x47],
            },
            "id=1;png_bytes=89504e47",
        ),
        vector(
            &DrawSprite {
                id: 1,
                dst_x: -4,
                dst_y: 8,
                dst_w: 16,
                dst_h: 32,
                src_x: 0,
                src_y: 0,
                src_w: 16,
                src_h: 16,
                layer: 3,
                angle: 90.0,
                flip_h: true,
                flip_v: false,
                tint: (255, 255, 255, 128),
            },
            "id=1;dst_x=-4;dst_y=8;dst_w=16;dst_h=32;src_x=0;src_y=0;src_w=16;src_h=16;layer=3;angle=90;flip_h=1;flip_v=0;tint_r=255;tint_g=255;tint_b=255;tint_a=128",
        ),
        vector(
            &DrawRect {
                x: 10,
                y: 20,
                w: 100,
                h: 50,
                filled: true,
                color: (255, 0, 0, 255),
                layer: 2,
                screen_space: false,
            },
            "x=10;y=20;w=100;h=50;filled=1;r=255;g=0;b=0;a=255;layer=2;screen_space=0",
        ),
        vector(
            &DrawLine {
                x1: 0,
                y1: 0,
                x2: -10,
                y2: 10,
                color: (0, 255, 0, 255),
                layer: 0,
            },
            "x1=0;y1=0;x2=-10;y2=10;r=0;g=255;b=0;a=255;layer=0",
        ),
        vector(
            &DrawCircle {
                x: 5,
                y: 5,
                radius: 9,
                filled: false,
                color: (0, 0, 255, 255),
                layer: 1,
            },
            "x=5;y=5;radius=9;filled=0;r=0;g=0;b=255;a=255;layer=1",
        ),
        vector(
            &DrawTriangle {
                x1: 0,
                y1: 0,
                x2: 4,
                y2: 0,
                x3: 2,
                y3: 4,
                filled: true,
                color: (1, 2, 3, 4),
                layer: 5,
            },
            "x1=0;y1=0;x2=4;y2=0;x3=2;y3=4;filled=1;r=1;g=2;b=3;a=4;layer=5",
        ),
        vector(
            &DrawText {
                text: "hi",
                x: -1,
                y: 2,
                size: 14,
                color: (255, 255, 255, 255),
                layer: 7,
                screen_space: true,
                align: TextAlign::Center,
            },
            "text=hi;x=-1;y=2;size=14;r=255;g=255;b=255;a=255;layer=7;screen_space=1;align=1",
        ),
        vector(
            &SetCamera {
                x: 1.5,
                y: -2.5,
                zoom: 2.0,
            },
            "x=1.5;y=-2.5;zoom=2",
        ),
        vector(
            &SetDisplay {
                width: 1280,
                height: 720,
                fullscreen: false,
            },
            "width=1280;height=720;fullscreen=0",
        ),
        // -- renderer ---------------------------------------------------
        vector(
            &DisplayChanged {
                width: 1920,
                height: 1080,
            },
            "width=1920;height=1080",
        ),
        vector(
            &LogicalCanvas {
                width: 320,
                height: 180,
            },
            "width=320;height=180",
        ),
        // -- ui ---------------------------------------------------------
        vector(
            &Spec {
                title: "notes",
                widgets: vec![
                    Widget::Label { text: "name" },
                    Widget::TextEdit {
                        id: 1,
                        text: "ada",
                    },
                    Widget::Button {
                        id: 2,
                        label: "save",
                    },
                ],
            },
            "title=notes;widgets=label(name),text-edit(1,ada),button(2,save)",
        ),
        vector(&Spec { title: "", widgets: vec![] }, "title=;widgets="),
        vector(&Clicked { id: 2 }, "id=2"),
        vector(
            &Changed {
                id: 1,
                text: "ada l",
            },
            "id=1;text=ada l",
        ),
        // -- audio ------------------------------------------------------
        vector(
            &LoadSound {
                id: 4,
                bytes: &[0x52, 0x49, 0x46, 0x46],
            },
            "id=4;bytes=52494646",
        ),
        vector(
            &PlaySound {
                id: 4,
                volume: 0.75,
            },
            "id=4;volume=0.75",
        ),
        vector(
            &PlayMusic {
                id: 5,
                volume: 1.0,
            },
            "id=5;volume=1",
        ),
        vector(&StopMusic, "(no fields)"),
        vector(&SetMusicVolume { volume: 0.25 }, "volume=0.25"),
        // -- game-core --------------------------------------------------
        vector(
            &EntityOpMessage(EntityOp::Spawn {
                entity_id: 1,
                x: 32.0,
                y: 64.0,
                sprite: Some(Sprite {
                    sprite_id: 9,
                    frame_w: 16,
                    frame_h: 16,
                    frame_count: 4,
                    frame_duration: 0.1,
                }),
                square_color: (255, 0, 255, 255),
                shape: Shape::Rect,
                collider_half_w: 8.0,
                collider_half_h: 8.0,
                body_kind: BodyKind::Dynamic,
                worlds: PhysicsWorlds::RAPIER2D,
            }),
            "tag=0;entity_id=1;x=32;y=64;has_sprite=1;sprite_id=9;frame_w=16;frame_h=16;frame_count=4;frame_duration=0.1;r=255;g=0;b=255;a=255;shape=0;collider_half_w=8;collider_half_h=8;body_kind=0;worlds=1",
        ),
        vector(
            &EntityOpMessage(EntityOp::Spawn {
                entity_id: 2,
                x: 0.0,
                y: 0.0,
                sprite: None,
                square_color: (10, 20, 30, 255),
                shape: Shape::Triangle,
                collider_half_w: 4.0,
                collider_half_h: 4.0,
                body_kind: BodyKind::Fixed,
                worlds: PhysicsWorlds::BOTH,
            }),
            "tag=0;entity_id=2;x=0;y=0;has_sprite=0;sprite_id=0;frame_w=0;frame_h=0;frame_count=0;frame_duration=0;r=10;g=20;b=30;a=255;shape=1;collider_half_w=4;collider_half_h=4;body_kind=3;worlds=3",
        ),
        vector(
            &EntityOpMessage(EntityOp::SetVelocity {
                entity_id: 1,
                vx: -50.0,
                vy: 0.0,
            }),
            "tag=1;entity_id=1;vx=-50;vy=0",
        ),
        vector(
            &EntityOpMessage(EntityOp::Despawn { entity_id: 1 }),
            "tag=2;entity_id=1",
        ),
        vector(
            &EntityOpMessage(EntityOp::SetColor {
                entity_id: 1,
                color: (1, 2, 3, 4),
            }),
            "tag=3;entity_id=1;r=1;g=2;b=3;a=4",
        ),
        vector(
            &EntityOpMessage(EntityOp::SetDebugHitboxes { enabled: true }),
            "tag=4;enabled=1",
        ),
        vector(
            &EntityOpMessage(EntityOp::SetPaused { paused: false }),
            "tag=5;paused=0",
        ),
        vector(
            &EntityOpMessage(EntityOp::SetCameraFollow {
                entity_id: 1,
                viewport_w: 320.0,
                viewport_h: 180.0,
                zoom: 2.0,
            }),
            "tag=6;entity_id=1;viewport_w=320;viewport_h=180;zoom=2",
        ),
        vector(
            &EntityOpMessage(EntityOp::SetSprite {
                entity_id: 1,
                presentation: SpritePresentation {
                    sprite: Sprite {
                        sprite_id: 9,
                        frame_w: 16,
                        frame_h: 16,
                        frame_count: 4,
                        frame_duration: 0.1,
                    },
                    frames_per_row: 4,
                    draw_w: 32,
                    draw_h: 32,
                    looping: true,
                    advance_while_stopped: false,
                    flip_h: false,
                    flip_v: false,
                },
            }),
            "tag=7;entity_id=1;sprite_id=9;frame_w=16;frame_h=16;frame_count=4;frame_duration=0.1;frames_per_row=4;draw_w=32;draw_h=32;looping=1;advance_while_stopped=0;flip_h=0;flip_v=0",
        ),
        vector(
            &EntityOpMessage(EntityOp::SetCameraSmoothing {
                responsiveness: 8.0,
            }),
            "tag=8;responsiveness=8",
        ),
        vector(&EntityOpMessage(EntityOp::Reset), "tag=9"),
        vector(
            &EntityOpMessage(EntityOp::SetSpriteTint {
                entity_id: 1,
                tint: (255, 128, 0, 255),
            }),
            "tag=10;entity_id=1;r=255;g=128;b=0;a=255",
        ),
        vector(
            &EntityTransform {
                entity_id: 3,
                x: 1.25,
                y: -1.25,
            },
            "entity_id=3;x=1.25;y=-1.25",
        ),
        vector(
            &Collision {
                entity_id_a: 1,
                entity_id_b: 2,
            },
            "entity_id_a=1;entity_id_b=2",
        ),
        vector(
            &LoadTilemap {
                tmx_bytes: &[0x3c, 0x3f, 0x78, 0x6d],
                tileset_images: vec![TilesetImage {
                    name: "tiles",
                    sprite_id: 1,
                    png_bytes: &[0x89, 0x50],
                }],
            },
            "tmx_bytes=3c3f786d;count=1;images=tiles(1,8950)",
        ),
        // -- web --------------------------------------------------------
        vector(
            &PanelOpened {
                owner: "dashboard",
                panel: "main",
            },
            "owner=dashboard;panel=main",
        ),
        vector(
            &PanelClosed {
                owner: "dashboard",
                panel: "main",
            },
            "owner=dashboard;panel=main",
        ),
        vector(
            &PanelFailed {
                owner: "dashboard",
                panel: "main",
                reason: "no webview",
            },
            "owner=dashboard;panel=main;reason=no webview",
        ),
        vector(
            &PageMessage {
                owner: "dashboard",
                panel: "main",
                json: "{\"a\":1}",
            },
            "owner=dashboard;panel=main;json={\"a\":1}",
        ),
        // -- persistence ------------------------------------------------
        vector(&Save { bytes: &[1, 2, 3] }, "bytes=010203"),
        // -- direct sends -----------------------------------------------
        send_vector(
            "web",
            WebCommand::Open(OpenPanel {
                panel: "main",
                source: PanelSource::Html("<p>hi</p>"),
            })
            .encode(),
            "tag=0;panel=main;source_kind=0;source=<p>hi</p>",
        ),
        send_vector(
            "web",
            WebCommand::Open(OpenPanel {
                panel: "main",
                source: PanelSource::Url("https://example.test"),
            })
            .encode(),
            "tag=0;panel=main;source_kind=1;source=https://example.test",
        ),
        send_vector(
            "web",
            WebCommand::Close(ClosePanel { panel: "main" }).encode(),
            "tag=1;panel=main",
        ),
        send_vector(
            "web",
            WebCommand::Navigate(Navigate {
                panel: "main",
                url: "https://example.test/next",
            })
            .encode(),
            "tag=2;panel=main;url=https://example.test/next",
        ),
        send_vector(
            "web",
            WebCommand::SendJson(SendJson {
                panel: "main",
                json: "{\"b\":2}",
            })
            .encode(),
            "tag=3;panel=main;json={\"b\":2}",
        ),
        send_vector("persistence", Vec::new(), "(empty request: load)"),
        send_vector("files", b"config/theme.json".to_vec(), "path=config/theme.json"),
    ]
}

/// The shipped file, rendered.
///
/// Deliberately a flat, line-oriented format rather than JSON: an
/// implementation checking these vectors may be written in a language whose
/// standard library has no JSON parser, and this crate itself has no
/// dependencies to parse one with either.
fn render(vectors: &[Vector]) -> String {
    let mut out = String::new();
    out.push_str("# bones core message wire format -- conformance vectors\n");
    out.push_str("# version 1.0.0, matching bones:core@1.0.0 and wit/wire-format.md\n");
    out.push_str("#\n");
    out.push_str("# Generated from bones-messages. Do not edit by hand: crates/bones-messages/\n");
    out.push_str("# tests/conformance.rs fails when this file and the encoder disagree.\n");
    out.push_str("#\n");
    out.push_str("# One vector per line, three fields separated by a single space:\n");
    out.push_str("#   <topic-or-endpoint> <payload-as-lowercase-hex> <field>=<value>;...\n");
    out.push_str("#\n");
    out.push_str("# A payload of `-` is the empty payload, zero bytes. Byte-slice values are\n");
    out.push_str("# themselves written as hex. An implementation is conformant when it encodes\n");
    out.push_str("# each listed value to exactly the listed bytes, and decodes those bytes back\n");
    out.push_str("# to the listed values.\n");
    for vector in vectors {
        out.push('\n');
        let _ = writeln!(out, "{} {} {}", vector.topic, vector.hex, vector.fields);
    }
    out
}

fn vectors_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wit/vectors/vectors.txt")
}

#[test]
fn the_shipped_vectors_match_what_this_crate_encodes() {
    let rendered = render(&vectors());
    let path = vectors_path();

    if std::env::var_os("BONES_WRITE_VECTORS").is_some() {
        std::fs::create_dir_all(path.parent().expect("the path has a parent"))
            .expect("creating wit/vectors");
        std::fs::write(&path, &rendered).expect("writing the vectors");
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "cannot read {}: {err}. Generate it with BONES_WRITE_VECTORS=1 cargo test --test conformance",
            path.display()
        )
    });

    // Compared line by line so a failure names the topic that changed, rather
    // than printing two eight-kilobyte strings at each other.
    for (line, (committed, rendered)) in committed.lines().zip(rendered.lines()).enumerate() {
        assert_eq!(
            committed,
            rendered,
            "wit/vectors/vectors.txt line {} disagrees with this crate's encoding. \
             If the encoding changed on purpose, that is an ABI break: regenerate with \
             BONES_WRITE_VECTORS=1 cargo test --test conformance and bump the ABI version.",
            line + 1
        );
    }
    assert_eq!(
        committed.lines().count(),
        rendered.lines().count(),
        "wit/vectors/vectors.txt has a different number of vectors than this crate produces"
    );
}

#[test]
fn every_vector_is_well_formed() {
    for vector in vectors() {
        assert!(
            !vector.topic.contains(' '),
            "a topic with a space would break the file's one-line-per-vector format"
        );
        assert!(
            !vector.fields.contains('\n'),
            "{}: a field list must stay on one line",
            vector.topic
        );
        assert!(
            vector.hex == "-" || vector.hex.len() % 2 == 0,
            "{}: hex must be whole bytes",
            vector.topic
        );
    }
}
