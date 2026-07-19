//! Renderer module (design/modules.md, ADR-002, ADR-017): executes `gfx/*`
//! draw commands against one SDL window. A `bus::Module`: construction is
//! two-stage — `new` takes only a `Logger`, real SDL setup waits for
//! `init` to consume the `window-surface` service (the window itself,
//! provided by whoever configured one — `Engine::build` today).

mod circle_geometry;
mod inner;
mod renderer;
mod retained_draw;
mod text_rasterizer;
mod ui_mesh;

pub use renderer::Renderer;
pub use ui_mesh::{UiMesh, UiVertex};
