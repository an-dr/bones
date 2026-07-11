# platform

The only component touching the OS (design/platform.md). This rung: one SDL
window plus keyboard events onto `input/*` — ADR-008's web/egui layers don't
exist yet, so every event reaches `input/*` directly. Window/tray/mouse/
controller/timing are later work.

- `Platform::new(title, w, h)` — opens one SDL window.
- `Platform::poll_events(&bus, sender)` — publishes an `input/key-down` or
  `input/key-up` envelope per pending keyboard event (payload is the key
  name as UTF-8, e.g. `b"A"`). Only enqueues — the caller dispatches.

Builds SDL3 from source (`sdl3`'s `build-from-source-static` feature) — no
system SDL install needed. On Windows, if `cargo build` fails with a cmake
"could not create named generator" error, your cmake doesn't yet recognize
your Visual Studio version's generator name. Work around it: run from a
Developer Command Prompt (or call `vcvarsall.bat`) with `CMAKE_GENERATOR=Ninja`
set — Ninja just needs `cl.exe` on `PATH`, not a named VS generator.
