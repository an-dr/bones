//! Typed `gfx/*` draw commands shared by extensions and the renderer.

mod clear;
mod command;
mod draw_circle;
mod draw_line;
mod draw_rect;
mod draw_sprite;
mod draw_text;
mod load_sprite;
mod set_camera;

pub use clear::Clear;
pub use command::Command;
pub use draw_circle::DrawCircle;
pub use draw_line::DrawLine;
pub use draw_rect::DrawRect;
pub use draw_sprite::DrawSprite;
pub use draw_text::DrawText;
pub use load_sprite::LoadSprite;
pub use set_camera::SetCamera;

#[cfg(test)]
mod tests;
