# kernel infrastructure

The host infrastructure every native module depends *on*, merged into one crate because none of it orchestrates the optional modules the way `bones-kernel`'s own builder does (ADR-030). Always present: must build and run with no native module registered at all.

## bus

The message bus (ADR-003, ADR-009, ADR-013): pub/sub over hierarchical topics with exact and prefix-wildcard subscriptions, built on `pubsub-bus`.

- `Envelope` — topic, sender, payload.
- `Handler` — implemented by anything that receives envelopes.
- `Bus` — register/unregister endpoints, publish (enqueue), dispatch (deliver). Cheap to clone; every clone shares the same underlying bus.

Publish only enqueues; delivery happens on `dispatch()`, deferred so a handler can safely publish in response to what it just received (ADR-015).

`EndpointBudget` supplies ADR-007's per-frame inbound and publish allowances for untrusted endpoints. A bounded adapter drops matching deliveries over its allowance; the extension host in `bones-module-wasm` applies the same object before guest publishes. `Bus::begin_frame` resets every registered allowance while cumulative `DropCounters` and the violation flag remain observable; this crate's own `Runner::step` calls it at the authoritative frame boundary.

Direct send (ADR-010) addresses endpoints by name, separately from topic pub/sub: `Respond` is implemented by anything answerable via direct send, and `Registry::call(from, to, payload)` invokes `to` synchronously and returns its reply — a `to` already in the current call chain fails immediately with `SendError::Cycle` rather than deadlocking. `ModuleRegistration` attaches a native `Module` to both message paths at runtime and owns its shutdown/unregistration, supporting temporary compositions without rebuilding the engine.

## logging

Structured logging for the engine core. Every component logs through an injected `LogSink` (ADR-012) instead of calling `println!` directly, so the sink can be swapped without touching call sites.

- `Level` — Debug/Info/Warn/Error.
- `LogSink` — trait a sink implements; `StdoutSink` is the default.
- `Logger` — cheap-to-clone handle every component holds; forwards to its sink.
- `RecordingSink` — test double that captures calls instead of printing.

## platform

The only module touching the OS (design/platform.md), behind the `platform` feature. Owns one SDL window and turns everything it observes into bus messages, so nothing above it needs an OS API.

- `Platform::new(title, w, h)` — opens one resizable SDL window.
- `Platform::poll_events(&bus, sender)` — publishes one envelope per pending event onto `input/*`: keyboard, mouse, and gamepad. Payloads are the typed [`bones-messages`](../../bones-messages/README.md) `input` codecs. Only enqueues — the caller dispatches.
- `Platform::poll_events_with(...)` — the same, with a consumption hook so an upper layer can claim an event first (ADR-008: top layer consumes) — how the ui module takes keystrokes before they reach `input/*`.
- `display_modes()` / `native_display_mode()` — resolutions, queried once at startup. Empty or `None` means the query failed, not an error.
- `take_window()` / `provide_window(services)` / `reclaim_window(services)` — hand the window to a renderer or web panel through the service registry, and take it back (ADR-028).

Builds SDL3 from source (`build-from-source-static`), so no system SDL install is needed — but a C compiler and CMake are. If cmake fails with a "could not create named generator" error, build through `cargo xtask`, which configures the compiler and generator, instead of bare `cargo`.

## persistence and files

Bus endpoints any part may call, native or WASM, so they stay here rather than in the extension host (ADR-040):

- **`persistence`** — saves and restores an endpoint's own state, keyed by the sender, so an extension's state survives a reload. Unconditional, unlike the native modules — see the module's own doc comment for why.
- **`files`** — reads files inside one embedder-granted directory, mediated through the bus. Opt-in, and canonicalised on both sides so no `..`, absolute path, or symlink escapes it.

## services the kernel provides

- **`exit`** — `ExitRequest`, a shared flag any module sets to ask for orderly shutdown; the kernel checks it once per frame.
- **`display`** — `DisplayInfo`, the display's fullscreen modes and desktop resolution, queried once before any window hand-off.

Extensions are hosted by [`bones-module-wasm`](../bones-module-wasm/README.md), an opt-in module built on this crate. Nothing here depends on a WASM runtime.

## runner

The frame loop (ADR-014): `Runner::step(dt)` begins the frame, dispatches, ticks `core/tick` subscribers, and dispatches again. It lives here rather than in `bones-kernel` because it names no native module — it drives whatever was registered (ADR-031). Modules that need work on either side of the tick use the `before_tick` and `after_tick` hooks, which `BuiltKernel::step` runs. `bones-kernel` re-exports the runner; its own job is the builder that composes modules and the `BuiltKernel` it produces.
