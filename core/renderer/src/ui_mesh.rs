//! Renderer-agnostic shape for egui's tessellated output. Plain f32/u8
//! fields, no sdl3 types — the `ui` crate builds these without depending on
//! sdl3 itself, so swapping the SDL renderer for a future replacement (once
//! design/modules.md's `draw-target` service trait exists) only needs the
//! replacement to accept the same shape, not for `ui` to change.

/// One vertex of a textured triangle mesh: position and UV in the same
/// units the ui module produced them (physical pixels), color as straight
/// (non-premultiplied) RGBA.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiVertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// A texture-mapped triangle mesh with an axis-aligned scissor rect — the
/// shape egui's tessellator produces per clipped primitive.
#[derive(Debug, Clone, PartialEq)]
pub struct UiMesh {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
    /// Matches the id a prior `set_ui_texture` call registered.
    pub texture: u64,
    /// Scissor rect in physical pixels: (x, y, width, height).
    pub clip: (i32, i32, u32, u32),
}
