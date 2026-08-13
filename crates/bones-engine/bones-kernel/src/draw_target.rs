//! The `draw-target` service (design/modules.md): the contract a module
//! that owns a drawing surface offers to a module that has pixels to draw
//! but no surface of its own — `renderer` and `ui` today.
//!
//! It lives here, in the always-present tier, for the same reason
//! `window-surface` does: a service is a contract *between* two optional
//! modules, so neither of them can own it without the consumer having to
//! depend on the provider's crate — which is exactly the coupling the
//! service registry exists to remove.
//!
//! Nothing here is SDL-specific, or even renderer-specific. The types are
//! plain data and the trait is five methods, so a replacement surface
//! (a wgpu backend, a headless recorder for tests) satisfies the same
//! contract without `ui` changing at all.

/// One vertex of a textured triangle mesh: position and UV in the same
/// units the producing module used (physical pixels), color as straight
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

/// A surface that accepts textured meshes drawn above whatever it has
/// already composited this frame.
///
/// `Send` because the consumer holds one as a field and is itself
/// registered as a bus endpoint, which `bus::Handler` requires to be
/// `Send`. A provider whose real state is thread-affine (SDL's canvas is)
/// satisfies that the same way `renderer` already does internally, with a
/// `SendWrapper` that keeps the constraint enforced rather than assumed.
pub trait DrawTarget: Send {
    /// Current surface size in physical pixels, for a consumer sizing its
    /// own output to match without holding a window handle.
    fn size(&self) -> (u32, u32);

    /// Registers or fully replaces the RGBA8 (straight alpha) texture the
    /// consumer addresses as `id` in `UiMesh::texture`. `rgba.len()` must
    /// be `width * height * 4`.
    fn set_ui_texture(
        &mut self,
        id: u64,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String>;

    /// Patches a sub-rectangle of a texture already registered by
    /// `set_ui_texture` (e.g. a font atlas growing as new glyphs are
    /// rasterized). Errors if `id` was never `set_ui_texture`-created.
    fn update_ui_texture_region(
        &mut self,
        id: u64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String>;

    /// Drops a texture registered by `set_ui_texture`.
    fn free_ui_texture(&mut self, id: u64);

    /// Draws one textured triangle mesh above every batch composited this
    /// frame (design/presentation.md). The caller is responsible for
    /// calling this after the surface's own `render` phase has run.
    fn draw_ui_mesh(&mut self, mesh: &UiMesh) -> Result<(), String>;
}

/// The service value itself: `provide` one of these and the consumer
/// `consume`s it, exactly like `window-surface`.
///
/// A boxed trait object rather than the trait alone because
/// `ServiceRegistry` keys on a concrete `TypeId`, and both sides must name
/// the same one — `Box<dyn DrawTarget>` is that shared name.
pub type DrawTargetService = Box<dyn DrawTarget>;
