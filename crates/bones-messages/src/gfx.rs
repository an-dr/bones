//! Typed `gfx/*` draw commands shared by extensions and the renderer.

mod clear;
mod clear_draw_batch;
mod command;
mod draw_circle;
mod draw_line;
mod draw_rect;
mod draw_sprite;
mod draw_text;
mod draw_triangle;
mod load_sprite;
mod set_camera;
mod set_display;
mod text_align;

pub use clear::Clear;
pub use clear_draw_batch::ClearDrawBatch;
pub use command::Command;
pub use draw_circle::DrawCircle;
pub use draw_line::DrawLine;
pub use draw_rect::DrawRect;
pub use draw_sprite::DrawSprite;
pub use draw_text::DrawText;
pub use draw_triangle::DrawTriangle;
pub use load_sprite::LoadSprite;
pub use set_camera::SetCamera;
pub use set_display::SetDisplay;
pub use text_align::TextAlign;

#[cfg(test)]
mod tests;
