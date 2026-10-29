//! Renderer module (design/modules.md, ADR-002, ADR-017): executes `gfx/*`
//! draw commands against one SDL window. A `bones_kernel_infra::bus::Module`: construction is
//! two-stage — `new` takes only a `Logger`, real SDL setup waits for
//! `init` to consume the `window-surface` service (the window itself,
//! provided by whoever configured one — `Kernel::build` today).

mod circle_geometry;
mod frame_capture;
mod inner;
mod renderer;
mod retained_draw;
mod shared_inner;
mod text_alignment;
mod text_rasterizer;
mod window_capture;

pub use renderer::Renderer;
