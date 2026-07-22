# renderer

Executes `gfx/*` draw commands against one SDL window (design/modules.md,
ADR-002). Takes the window from `platform` (`Platform::take_window`) rather
than opening its own. Decodes with `bones-messages`'s typed `gfx` messages,
which also define the wire format and are used by extensions building
payloads (e.g. `sprite_demo`) get it from:

- `gfx/clear` — 4 bytes: `r, g, b, a`.
- `gfx/load-sprite` — 4-byte `u32` id (little-endian) + raw image bytes
  (PNG, via `sdl3`'s `image` feature); decodes once and caches the texture.
- `gfx/draw-sprite` — 28 bytes: `id, dst_x, dst_y, src_x, src_y, src_w,
  src_h` (`id`/`src_w`/`src_h` as `u32`, the rest `i32`, all little-endian).
  `src_*` crops a sub-rectangle out of the loaded texture — the same
  mechanism a sprite sheet's individual frames use.
- `gfx/draw-triangle` — three vertices, `filled`, `color`, `layer`. Drawn
  via `Canvas::render_geometry` (the same untextured triangle-mesh
  primitive `ui`'s egui output uses), not `fill_rect` — the only
  non-axis-aligned shape this renderer draws.

`Renderer` isn't `Send` on its own — SDL's `Window`/`Canvas` have real
thread-affinity constraints on some platforms — but the vendored
`pubsub-bus` crate requires it on anything registered as a bus endpoint.
Wrapped in `send_wrapper::SendWrapper`, which panics (not silent UB) if
ever actually touched from a different thread than it was created on; true
today since bus dispatch is single-threaded.
