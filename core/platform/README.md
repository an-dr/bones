# platform

The only component touching the OS (design/platform.md). Owns one SDL window
and turns everything it observes into bus messages, so nothing above it needs
an OS API.

- `Platform::new(title, w, h)` — opens one resizable SDL window.
- `Platform::set_min_size(w, h)` — floors how small it can be resized.
- `Platform::poll_events(&bus, sender)` — publishes one envelope per pending
  event onto `input/*`: keyboard (`KeyDown`/`KeyUp`), mouse
  (`MouseDown`/`MouseUp`/`MouseMove`/`MouseWheel`), and gamepad
  (`GamepadConnected`/`GamepadDisconnected`/`GamepadButtonDown`/
  `GamepadButtonUp`/`GamepadAxis`). Payloads are the typed
  [`bones-messages`](../../shared/bones-messages/README.md) `input` codecs.
  Only enqueues — the caller dispatches.
- `Platform::poll_events_with(...)` — the same, with a consumption hook so an
  upper layer can claim an event first (ADR-008: top layer consumes). This is
  how the ui module takes keystrokes before they reach `input/*`.
- `display_modes()` / `native_display_mode()` — resolutions, queried once at
  startup before any window hand-off. Empty or `None` means the query failed
  (no display attached), not an error: build your resolution picker with a
  fallback.
- `take_window()` / `provide_window(services)` / `reclaim_window(services)` —
  hand the window to a renderer or web panel through the service registry,
  and take it back. This is what lets presentation attach to and detach from
  a live engine (ADR-028).

## Constraints worth knowing

- **A gamepad only emits events while its handle stays open.** The crate
  holds one per connected device and drops it on removal; nothing above it
  needs to manage that.
- **Not everything reaches the bus yet.** Tray and timing sources are not
  published onto `input/*`, and text-input events are captured but only ever
  reach the ui module's consumption hook.
- **Builds SDL3 from source** (`sdl3`'s `build-from-source-static` feature),
  so no system SDL install is needed — but a C compiler and CMake are. Any
  compiler works (MSVC, clang, clang-cl, gcc); it is not MSVC-specific.

If cmake fails with a "could not create named generator" error, it did not
recognize your Visual Studio version's generator name. Use
[`scripts/native-build-env.ps1`](../../scripts/native-build-env.ps1), which
sets `CMAKE_GENERATOR=Ninja` when Ninja is available and falls back to loading
MSVC — it is what `dist.ps1` and `test.ps1` both use, so building through
those scripts avoids the problem entirely.
