# platform

The only component touching the OS (design/platform.md). Opens one SDL
window and publishes keyboard events onto `input/*` — ADR-008's web/egui
layers don't exist yet, so every event reaches `input/*` directly. Window/
tray/mouse/controller/timing aren't implemented yet.

- `Platform::new(title, w, h)` — opens one resizable SDL window.
- `Platform::set_min_size(w, h)` — floors how small that window can be resized.
- `Platform::poll_events(&bus, sender)` — publishes an `input/key-down` or
  `input/key-up` envelope per pending keyboard event (payload is the key
  name as UTF-8, e.g. `b"A"`). Only enqueues — the caller dispatches.

Builds SDL3 from source (`sdl3`'s `build-from-source-static` feature) — no
system SDL install needed, and any C compiler works (MSVC, clang, clang-cl,
gcc — not MSVC-specific). `dist.ps1` detects and uses whatever's already on
`PATH`, falling back to loading MSVC only if nothing else is found.

Building directly with `cargo build`/`cargo test` (bypassing `dist.ps1`) on
a machine with no compiler set up at all may hit a cmake "could not create
named generator" error if cmake doesn't recognize your Visual Studio
version's generator name. Work around it: run from a Developer Command
Prompt (or call `vcvarsall.bat`) with `CMAKE_GENERATOR=Ninja` set — Ninja
just needs a compiler on `PATH`, not a named VS generator.
