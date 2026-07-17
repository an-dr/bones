//! Renderer module (design/modules.md, ADR-002): executes `gfx/*` draw
//! commands against one SDL window. TODO: not a generic Module yet —
//! directly wired into Engine, same as host and platform, until the module
//! trait exists to design from real examples.

use std::collections::HashMap;

use bones_messages::gfx::Command;
use bus::{Envelope, Handler};
use logging::Logger;
use sdl3::image::LoadTexture;
use sdl3::pixels::Color;
use sdl3::rect::Rect;
use sdl3::render::{Canvas, Texture, TextureCreator};
use sdl3::video::{Window, WindowContext};
use send_wrapper::SendWrapper;

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
            Command::Clear(clear) => {
                let (r, g, b, a) = (clear.r, clear.g, clear.b, clear.a);
                self.canvas.set_draw_color(Color::RGBA(r, g, b, a));
                self.canvas.clear();
            }
            Command::LoadSprite(load) => {
                let texture = self
                    .texture_creator
                    .load_texture_bytes(load.png_bytes)
                    .map_err(|e| e.to_string())?;
                self.textures.insert(load.id, texture);
            }
            Command::DrawSprite(draw) => {
                let texture = self
                    .textures
                    .get(&draw.id)
                    .ok_or_else(|| format!("unknown sprite id {}", draw.id))?;
                let src = Rect::new(draw.src_x, draw.src_y, draw.src_w, draw.src_h);
                let dst = Rect::new(draw.dst_x, draw.dst_y, draw.src_w, draw.src_h);
                self.canvas
                    .copy(texture, src, dst)
                    .map_err(|e| e.to_string())?;
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
        // Only ever reached for gfx/* (the subscription), so an unmatched
        // topic here is always a caller mistake (e.g. a typo'd command
        // name) worth surfacing rather than silently dropping.
        let result = match Command::decode(&envelope.topic, &envelope.payload) {
            Ok(Some(command)) => self.0.execute(command),
            Ok(None) => {
                self.0.logger.warn(
                    "renderer",
                    &format!(
                        "unknown command '{}' from '{}'",
                        envelope.topic, envelope.sender
                    ),
                );
                return;
            }
            Err(err) => Err(err.to_string()),
        };
        if let Err(err) = result {
            self.0.logger.error(
                "renderer",
                &format!("{} from '{}': {err}", envelope.topic, envelope.sender),
            );
        }
    }
}
