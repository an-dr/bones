use std::sync::{Arc, Mutex, MutexGuard};

use bones_kernel::draw_target::{DrawTarget, UiMesh};
use send_wrapper::SendWrapper;

use crate::inner::Inner;

/// The SDL state, shared between `Renderer` itself and the
/// `draw-target` service it hands to whoever draws above it.
///
/// Two owners rather than one because the two access paths are genuinely
/// different: `Renderer` reaches this through bus dispatch (`gfx/*`
/// commands, the `render`/`present` phases), while the consumer of
/// `draw-target` reaches it from its own module hooks, holding no
/// reference to `Renderer` at all. Sharing the state instead of the
/// `Renderer` is what lets `ui` stop depending on this crate.
///
/// `SendWrapper` inside the `Mutex`, not outside: it is what makes the
/// whole handle `Send` (which `DrawTarget` requires) while keeping SDL's
/// real thread-affinity enforced — touching the canvas from another thread
/// panics rather than corrupting, exactly as `Renderer`'s own wrapper
/// already promised.
#[derive(Clone)]
pub(crate) struct SharedInner(Arc<Mutex<SendWrapper<Inner>>>);

impl SharedInner {
    pub(crate) fn new(inner: Inner) -> Self {
        Self(Arc::new(Mutex::new(SendWrapper::new(inner))))
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, SendWrapper<Inner>> {
        self.0.lock().expect("renderer state mutex poisoned")
    }
}

impl DrawTarget for SharedInner {
    fn size(&self) -> (u32, u32) {
        self.lock().canvas.window().size()
    }

    fn set_ui_texture(
        &mut self,
        id: u64,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        self.lock().set_ui_texture(id, width, height, rgba)
    }

    fn update_ui_texture_region(
        &mut self,
        id: u64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        self.lock()
            .update_ui_texture_region(id, x, y, width, height, rgba)
    }

    fn free_ui_texture(&mut self, id: u64) {
        self.lock().free_ui_texture(id);
    }

    fn draw_ui_mesh(&mut self, mesh: &UiMesh) -> Result<(), String> {
        self.lock().draw_ui_mesh(mesh)
    }
}
