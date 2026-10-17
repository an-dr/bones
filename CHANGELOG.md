# Changelog

Two independent version lines (ADR-029), so each entry says which one it belongs to:

- **engine** — `bones` and `bones-kernel`, tagged `v<version>`. What an embedder links.
- **ABI** — `bones:extension`, `bones-messages`, and `bones-wasm-sdk`, tagged `abi-v<version>`. What a `.wasm` extension is built against.

An ABI entry concerns every extension in existence, in any language. An engine entry concerns only projects that link the library.

## Unreleased

Repository tooling only; neither version line moves.

- **The build is `cargo xtask`.** `test.ps1`, `dist.ps1`, `scripts/native-build-env.ps1`, and every example's `build.ps1` are replaced by `cargo xtask test`, `cargo xtask dist`, and `cargo xtask example <name>`. PowerShell is no longer a prerequisite; the Rust toolchain that builds the engine runs its automation.
- **Prerequisites are checked, never installed.** Every command stops at the first missing tool with where to get it, and `cargo xtask doctor` reports them all at once.
- **`dashboard-demo` packages `metrics_demo.wasm`.** Its old script still built a `metrics` crate that no longer existed.

## engine 2.0.0 / ABI 1.2.0

What a desktop application built on bones needs around the engine, and should not write again: one window per user, a place in the application list, self-update on a page's request, starting programs and terminals, watching folders, and a channel a test can drive the real application through. Each is a leaf library an application's own code calls, plus a bus module only where a page or guest needs one.

`bones:extension` stays at 1.0.0. Nothing in `wit/` changed; the ABI line moves because `bones-messages` gained three modules.

### Breaking: separate launching from upgrading

`bones-upgrader` is removed. `bones-versioning` owns pure version semantics, `bones-launcher` selects and starts an installed version, and `bones-module-upgrade` owns downloading and installation (ADR-039). Enable the kernel's `upgrade` feature, use `bones_kernel::upgrade`, and compose `.upgrade(UpgradeConfig::new(identity, source, version))`. Trusted startup operations are methods of `UpgradeModule`; the internal update helpers are not a standalone library API. The `upgrader/*` wire contract is unchanged.

The launcher forwards application flags and never deletes versions. `--previous` selects a previous build for recovery; the application's upgrade module performs rollback. Hosts supply installation identity explicitly rather than reading it from a shared updater crate.

Versioning delegates strict SemVer parsing, precedence, and Cargo-style requirements to `semver`. Versions require all three core numbers. Collision folders now use `version~instance`; legacy `version-hash` folders are interpreted as prereleases. Build metadata alone does not trigger an update.

### Breaking: the kernel and the kernel infrastructure

"Engine" named both the product and its library, so the library and its core are renamed (ADR-038). Package boundaries are unchanged; only names move.

| Was | Is |
| --- | --- |
| `bones-engine` crate, `bones_engine::Engine`, `BuiltEngine` | `bones-kernel` crate, `bones_kernel::Kernel`, `BuiltKernel` |
| `bones-kernel` (the always-present core) | `bones-kernel-infra` |
| `crates/bones-engine/` | `crates/bones-kernel/` |

To adopt: depend on `bones-kernel` instead of `bones-engine`, replace `bones_engine::` with `bones_kernel::` and `Engine`/`BuiltEngine` with `Kernel`/`BuiltKernel`. Feature names and builder methods are unchanged. Nothing changes for extensions: the ABI line is not affected by the rename, and bus sender names stay as they were.

**The release archive ships no extension.** The former reference extension, `bones-extension-hello`, is now the example `examples/extensions/hello-demo`, where it remains the first-extension tutorial and the extension-host test fixture. `dist.ps1` ships the engine, the ABI, and an empty `extensions/` folder.

### Engine

Every new module is off by default: each is something an application chooses to be.

- **Single instance** (ADR-035). `bones-module-single-instance` holds both halves: `claim`, which claims the application's per-user `app_uid` in `main`, or hands the launch's arguments and working directory to the process that holds it — a named pipe on Windows, a Unix domain socket elsewhere. A crashed instance is taken over; one that does not answer within the timeout is given up on. Its module publishes each later launch as `instance/launched` and raises the window. To adopt: enable the `instance` feature, claim in `main`, and pass the `Primary` to `.single_instance(primary)`:

  ```rust
  let handoff = Handoff { args: std::env::args().skip(1).collect(), cwd: std::env::current_dir()? };
  match bones_kernel::instance::claim("myapp", handoff, Duration::from_secs(2))? {
      Claim::HandedOff => return Ok(()),
      Claim::Primary(primary) => Kernel::new().window("myapp", 800, 600).web().single_instance(primary).run()?,
  }
  ```

- **The `window-control` service.** The platform provides `WindowControl::raise()` beside `window-surface`, so a module can bring the window forward without owning it. Re-exported as `bones_kernel::window_control`.
- **Desktop registration** (ADR-036). `bones-module-desktop` is Commits' `desktop-integration`, moved: the same files, registry keys, and labels, so an existing registration still reads as registered. `Integration` gains `categories`, the freedesktop menu categories that were hard-coded. Its plain functions serve an installer and its module answers `desktop/*` for one `Integration` the composition fixes. To adopt: enable `desktop` and call `bones_kernel::desktop::register` from `--install`, after `bones-upgrader` placed the files; for a page's "Register with system", enable `desktop` and add `.desktop(integration)`.
- **The update flow and release convention** (ADR-037). `bones_upgrader::updater` holds `check`, `stage`, `install`, and `mark_executables` as plain functions; `ReleaseSource` names the latest GitHub release's `<app>-<platform>.json`, or an override URL. `bones-module-upgrader` answers `upgrader/*` off the frame thread with its own HTTPS download (256 MB, per-read timeouts), and reports the version that ran before this one. The source is fixed at composition; a guest chooses the action, never the URL. To adopt: enable `upgrader` and add `.upgrader(UpgraderConfig::new(ReleaseSource::with_override(github, &settings.override), VERSION))`.
- **Companion installs.** `bones_upgrader::install_companion` installs another bones application from its own release archive, unless it is already there (`Policy::IfAbsent`) or unless it is older (`Policy::IfNewer`). `install_fresh` now also finds a launcher under its payload name, which is how an unpacked release archive carries it.
- **Launching programs.** `bones-kernel-launch` resolves a program on `PATH` (and `PATHEXT` on Windows), starts it detached in a folder, and opens terminals from a table of known ones. No bus. To adopt: enable the kernel's `launch` feature and call `bones_kernel::launch::{resolve, launch, open_terminal}` from native code.
- **Watching folders.** `bones-kernel-watch` reports one debounced `Change` per watch per burst, drops access events on every platform, and runs on its own threads. No bus. To adopt: enable the kernel's `watch` feature and use `bones_kernel::watch::Watcher`.
- **Devtools channel.** `bones-module-devtools` is Commits' loopback HTTP channel, moved: post a page message, read the recorded bus. Inert unless the variable the composition names holds a port; `redacting(predicate)` keeps secrets out of the recording. To adopt: enable `control` and add `.devtools_from_env("MYAPP_DEVTOOLS_PORT")`.
- **X11 without touching the environment.** With `.web()` on Linux, the engine selects SDL's X11 video driver through an SDL hint, as GTK's backend already was through GDK, so an application no longer exports `SDL_VIDEO_DRIVER` or `GDK_BACKEND` for itself and children inherit the user's own values. Not yet measured on a Wayland session.
- **Examples.** [single-instance-app](examples/embedding/single-instance-app/README.md) is new. [self-updating-app](examples/embedding/self-updating-app/README.md) now updates through the `upgrader` module and has an `--install` that registers with the desktop.

Not in this release: serving an `Html` panel while the frame thread is busy. Measured on Windows, no way of serving the page loads before the thread resumes; [roadmap.md](docs/roadmap.md) has the evidence and the two ways forward.

### ABI

- `bones-messages::instance`: `Launched` on `instance/launched`.
- `bones-messages::desktop`: `Request` and `Result` on `desktop/*`.
- `bones-messages::upgrader`: `Request` and `Result` on `upgrader/*`.

All three are specified in [wit/wire-format.md](wit/wire-format.md) and carry conformance vectors.

## engine 1.1.1

`bones:extension` and `bones-messages` are unchanged, so there is no ABI entry.

### Engine

- `bones-module-web`: fixed linux webview crashes and black windows -- initialize GTK on the SDL runner thread before wry creates child views, select X11 through GDK APIs without changing the process environment, reject incompatible GTK threads and displays before wry can panic, and drain bounded GLib work each frame so pages render and exchange IPC.

## engine 1.1.0 / ABI 1.1.0

Two capabilities every desktop application needs and none of them should write again: self-update, and the OS surface a sandboxed extension cannot reach.

`bones:extension` stays at 1.0.0. Nothing in `wit/` changed, so every extension in the field still instantiates -- the ABI line moves because `bones-messages` gained a module, not because the guest contract did.

### Engine

- `bones-upgrader`: self-update for an application shipping outside a store -- manifest fetch, version comparison, version folders, a permanent launcher, staged installs and rollback. The four host-specific names arrive at compile time through `option_env!`, so the crate spells no application into itself. Its one host requirement is `Fetch::fetch_url`; everything else is filesystem and version arithmetic. It is called `upgrader` because Windows auto-elevates executables whose filename contains `update`, `install`, `setup` or `patch` (ADR-033).
- `bones-module-os`, behind the engine's `os` feature and `.os()` on the builder: clipboard, browser, native file dialogs and HTTPS fetch, performed on a guest's behalf. Off by default, because it pulls in a TLS stack and native dialogs a headless build has no use for. `OsBackend` is the seam, so a host can substitute one that opens no dialogs.
- [examples/embedding/self-updating-app](examples/embedding/self-updating-app/README.md): an application that replaces itself, and the launcher that starts whichever version is current. It builds two versions so the update is real rather than a check reporting "up to date", and covers rollback.

### ABI

- `bones-messages::os`: `Request` and `Result` on `os/*`, the typed vocabulary for the module above.
- `Writer::try_str` and `Writer::try_blob`: `str` and `blob` state their length limits as preconditions and panic, which suits the engine-controlled values the other messages carry. A host encoding a filesystem path or clipboard text did not choose the length, and needs it reported rather than asserted. They encode identical bytes.

## engine 1.0.0 / ABI 1.0.0

First tagged release. Both lines start at 1.0.0 (ADR-029) because the interfaces are settled, not because they arrived together; they are expected to diverge from here.

### The two public surfaces

- `bones-engine` is the one crate an embedder depends on, and the dependency graph enforces it: `bones`, the shipped executable, depends on nothing else, so it has no access an embedder lacks (ADR-030).
- Every type the facade's own signatures mention is reachable through it — the raw platform event, the error and result, the platform value, the window and draw-target services — so an embedder never adds `sdl3`, `wasmtime`, or `bones-kernel` to a manifest to satisfy a signature.
- Every public item on `bones-engine` is documented, and `missing_docs` is enabled so a new one cannot arrive without it.
- `BuiltEngine`'s fields are public and stable. A custom driver needs them all at once and takes them by destructuring.

### The extension ABI

- [wit/extension.wit](wit/extension.wit) defines the calls; [wit/wire-format.md](wit/wire-format.md) defines the bytes those calls carry on core-owned topics, with machine-readable [conformance vectors](wit/vectors/README.md) generated from the engine's own encoder. An extension author outside Rust no longer has to read Rust source to build a payload.
- A distribution ships the whole `wit/` directory beside the binary that implements it.
- Version enforcement is not advisory: wasmtime refuses to instantiate a component whose imported interface version differs from the host's.

### Behaviour

- **Input is offered topmost-first.** Modules are offered raw platform events in reverse registration order, so the module drawn above another is asked first, as ADR-008 requires (ADR-031). Previously the earliest-registered module — the renderer, drawn underneath everything — got the first look, so an interactive overlay could not claim a click landing on it.
- **An invalid `tick_hz` is an error, not a panic.** Zero, negative, NaN, infinity, and rates too small to have a representable frame period are rejected by `build` and `run` through the `Result` they already return.

### Distribution

- 1.0 is distributed by git tag. Every package carries `publish = false` and states why it cannot produce a self-contained registry archive; crates.io stays open as a later decision rather than an implied promise.
- `dist.ps1` produces a versioned, per-platform archive with a SHA256 beside it, containing the engine, the reference extension, the ABI, a sample `bones.toml`, `LICENSE`, and generated third-party notices.

### Examples

- `examples/` splits into `extensions/` and `embedding/`, so which of the two ways to use bones an example demonstrates is visible before opening it.
- Every example is kebab-case ending in `-demo`. The suffix is load-bearing rather than decorative: an extension's bus endpoint name is the stem of its `.wasm`, and `audio`, `game-core`, and `persistence` are already native module endpoints.
- The thin `embedding-demo` is replaced by `embedding/custom-engine`, which produces an actual executable: the shipped composition plus a native module, a message vocabulary bones does not own, and a WASM extension speaking it from the other side of the sandbox. It is the first example of defining custom messages at all.

### Development

- `test.ps1` is the whole suite: fixtures, formatting, clippy with warnings denied, the default and all-feature test runs, and the documentation build.
- The web-enabled integration tests pass as a suite. They previously aborted when run together, because SDL treats the first thread to initialise it as its main thread and libtest gives every test a fresh one; the SDL-touching tests now share one owning thread.

## Before 1.0

The granular pre-release history — 176 commits from 2026-07-10 to 2026-08-09 — is archived at the tag `compat-main-2026-08-09`. The phase commits on `main` are squashed from it, and no version line existed before this release.
