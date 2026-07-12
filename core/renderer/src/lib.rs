//! Renderer module (design/modules.md, ADR-002): executes `gfx/*` draw
//! commands against one SDL window. TODO: not a generic Module yet —
//! directly wired into Engine, same as host and platform, until the module
//! trait exists to design from real examples.

use std::collections::HashMap;

use bus::{Envelope, Handler};
use logging::Logger;
use send_wrapper::SendWrapper;
use sdl3::image::LoadTexture;
use sdl3::pixels::Color;
use sdl3::rect::Rect;
use sdl3::render::{Canvas, Texture, TextureCreator};
use sdl3::video::{Window, WindowContext};

const CLEAR_TOPIC: &str = "gfx/clear";
const LOAD_SPRITE_TOPIC: &str = "gfx/load-sprite";
const DRAW_SPRITE_TOPIC: &str = "gfx/draw-sprite";

enum Command<'a> {
    Clear { r: u8, g: u8, b: u8, a: u8 },
    LoadSprite { id: u32, png_bytes: &'a [u8] },
    DrawSprite {
        id: u32,
        dst_x: i32,
        dst_y: i32,
        src_x: i32,
        src_y: i32,
        src_w: u32,
        src_h: u32,
    },
}

fn parse_command<'a>(topic: &str, payload: &'a [u8]) -> Option<Result<Command<'a>, String>> {
    match topic {
        CLEAR_TOPIC => Some(parse_clear(payload)),
        LOAD_SPRITE_TOPIC => Some(parse_load_sprite(payload)),
        DRAW_SPRITE_TOPIC => Some(parse_draw_sprite(payload)),
        _ => None,
    }
}

fn parse_clear(payload: &[u8]) -> Result<Command<'_>, String> {
    let mut r = wire::Reader::new(payload);
    let red = r.read_u8().map_err(|e| e.to_string())?;
    let green = r.read_u8().map_err(|e| e.to_string())?;
    let blue = r.read_u8().map_err(|e| e.to_string())?;
    let alpha = r.read_u8().map_err(|e| e.to_string())?;
    r.finish().map_err(|e| e.to_string())?;
    Ok(Command::Clear { r: red, g: green, b: blue, a: alpha })
}

fn parse_load_sprite(payload: &[u8]) -> Result<Command<'_>, String> {
    let mut r = wire::Reader::new(payload);
    let id = r.read_u32().map_err(|e| e.to_string())?;
    Ok(Command::LoadSprite { id, png_bytes: r.read_rest() })
}

fn parse_draw_sprite(payload: &[u8]) -> Result<Command<'_>, String> {
    let mut r = wire::Reader::new(payload);
    let id = r.read_u32().map_err(|e| e.to_string())?;
    let dst_x = r.read_i32().map_err(|e| e.to_string())?;
    let dst_y = r.read_i32().map_err(|e| e.to_string())?;
    let src_x = r.read_i32().map_err(|e| e.to_string())?;
    let src_y = r.read_i32().map_err(|e| e.to_string())?;
    let src_w = r.read_u32().map_err(|e| e.to_string())?;
    let src_h = r.read_u32().map_err(|e| e.to_string())?;
    r.finish().map_err(|e| e.to_string())?;
    Ok(Command::DrawSprite { id, dst_x, dst_y, src_x, src_y, src_w, src_h })
}

struct Inner {
    canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
    // `unsafe_textures` (sdl3 feature): a `Texture<'r>` borrowing from
    // `texture_creator` can't be cached in the same struct as its own
    // creator without it — the crate's sanctioned way to keep textures
    // across frames instead of a self-referential struct.
    textures: HashMap<u32, Texture>,
    logger: Logger,
}

impl Inner {
    fn execute(&mut self, command: Command) -> Result<(), String> {
        match command {
            Command::Clear { r, g, b, a } => {
                self.canvas.set_draw_color(Color::RGBA(r, g, b, a));
                self.canvas.clear();
            }
            Command::LoadSprite { id, png_bytes } => {
                let texture = self
                    .texture_creator
                    .load_texture_bytes(png_bytes)
                    .map_err(|e| e.to_string())?;
                self.textures.insert(id, texture);
            }
            Command::DrawSprite {
                id,
                dst_x,
                dst_y,
                src_x,
                src_y,
                src_w,
                src_h,
            } => {
                let texture = self
                    .textures
                    .get(&id)
                    .ok_or_else(|| format!("unknown sprite id {id}"))?;
                let src = Rect::new(src_x, src_y, src_w, src_h);
                let dst = Rect::new(dst_x, dst_y, src_w, src_h);
                self.canvas.copy(texture, src, dst).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

/// SDL's `Window`/`Canvas` aren't `Send`/`Sync` (real thread-affinity
/// constraints on some platforms), but the vendored `pubsub-bus` crate
/// requires both on anything registered as a bus endpoint. `SendWrapper`
/// makes the type check pass while keeping the guarantee real: it panics
/// (not silent UB) if `Inner` is ever actually touched from a thread other
/// than the one that created it — true today, since dispatch is single-
/// threaded, but enforced rather than merely assumed.
pub struct Renderer(SendWrapper<Inner>);

impl Renderer {
    pub fn new(window: Window, logger: Logger) -> Self {
        let canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        Self(SendWrapper::new(Inner {
            canvas,
            texture_creator,
            textures: HashMap::new(),
            logger,
        }))
    }

    pub fn present(&mut self) {
        self.0.canvas.present();
    }
}

impl Handler for Renderer {
    fn handle(&mut self, envelope: &Envelope) {
        let Some(parsed) = parse_command(&envelope.topic, &envelope.payload) else {
            return;
        };
        let result = parsed.and_then(|command| self.0.execute(command));
        if let Err(err) = result {
            self.0
                .logger
                .error("renderer", &format!("{} from '{}': {err}", envelope.topic, envelope.sender));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_gfx_topics_are_ignored() {
        assert!(parse_command("input/key-down", b"whatever").is_none());
    }

    #[test]
    fn clear_parses_rgba_bytes() {
        let Some(Ok(Command::Clear { r, g, b, a })) = parse_command(CLEAR_TOPIC, &[10, 20, 30, 255]) else {
            panic!("expected a parsed Clear command");
        };
        assert_eq!((r, g, b, a), (10, 20, 30, 255));
    }

    #[test]
    fn clear_rejects_the_wrong_byte_count() {
        assert!(parse_command(CLEAR_TOPIC, &[1, 2, 3]).unwrap().is_err());
    }

    #[test]
    fn load_sprite_splits_id_from_the_remaining_bytes() {
        let mut payload = 7u32.to_le_bytes().to_vec();
        payload.extend_from_slice(b"not-really-a-png");

        let Some(Ok(Command::LoadSprite { id, png_bytes })) = parse_command(LOAD_SPRITE_TOPIC, &payload) else {
            panic!("expected a parsed LoadSprite command");
        };
        assert_eq!(id, 7);
        assert_eq!(png_bytes, b"not-really-a-png");
    }

    #[test]
    fn draw_sprite_parses_every_field_in_order() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&7u32.to_le_bytes()); // id
        payload.extend_from_slice(&100i32.to_le_bytes()); // dst_x
        payload.extend_from_slice(&200i32.to_le_bytes()); // dst_y
        payload.extend_from_slice(&0i32.to_le_bytes()); // src_x
        payload.extend_from_slice(&0i32.to_le_bytes()); // src_y
        payload.extend_from_slice(&64u32.to_le_bytes()); // src_w
        payload.extend_from_slice(&64u32.to_le_bytes()); // src_h

        let Some(Ok(Command::DrawSprite {
            id,
            dst_x,
            dst_y,
            src_x,
            src_y,
            src_w,
            src_h,
        })) = parse_command(DRAW_SPRITE_TOPIC, &payload)
        else {
            panic!("expected a parsed DrawSprite command");
        };
        assert_eq!((id, dst_x, dst_y, src_x, src_y, src_w, src_h), (7, 100, 200, 0, 0, 64, 64));
    }

    #[test]
    fn draw_sprite_rejects_the_wrong_byte_count() {
        assert!(parse_command(DRAW_SPRITE_TOPIC, &[0; 27]).unwrap().is_err());
    }
}
