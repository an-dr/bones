# Changelog

Two independent version lines (ADR-029), so each entry says which one it belongs to:

- **engine** — `bones` and `bones-engine`, tagged `v<version>`. What an embedder links.
- **ABI** — `bones:core`, `bones-messages`, and `bones-wasm-sdk`, tagged `abi-v<version>`. What a `.wasm` extension is built against.

An ABI entry concerns every extension in existence, in any language. An engine entry concerns only projects that link the library.

## engine 1.0.0 / ABI 1.0.0

First tagged release. Both lines start at 1.0.0 (ADR-029) because the interfaces are settled, not because they arrived together; they are expected to diverge from here.

### The two public surfaces

- `bones-engine` is the one crate an embedder depends on, and the dependency graph enforces it: `bones`, the shipped executable, depends on nothing else, so it has no access an embedder lacks (ADR-030).
- Every type the facade's own signatures mention is reachable through it — the raw platform event, the error and result, the platform value, the window and draw-target services — so an embedder never adds `sdl3`, `wasmtime`, or `bones-kernel` to a manifest to satisfy a signature.
- Every public item on `bones-engine` is documented, and `missing_docs` is enabled so a new one cannot arrive without it.
- `BuiltEngine`'s fields are public and stable. A custom driver needs them all at once and takes them by destructuring.

### The extension ABI

- [wit/core.wit](wit/core.wit) defines the calls; [wit/wire-format.md](wit/wire-format.md) defines the bytes those calls carry on core-owned topics, with machine-readable [conformance vectors](wit/vectors/README.md) generated from the engine's own encoder. An extension author outside Rust no longer has to read Rust source to build a payload.
- A distribution ships the whole `wit/` directory beside the binary that implements it.
- Version enforcement is not advisory: wasmtime refuses to instantiate a component whose imported interface version differs from the host's.

### Behaviour

- **Input is offered topmost-first.** Modules are offered raw platform events in reverse registration order, so the module drawn above another is asked first, as ADR-008 requires (ADR-031). Previously the earliest-registered module — the renderer, drawn underneath everything — got the first look, so an interactive overlay could not claim a click landing on it.
- **An invalid `tick_hz` is an error, not a panic.** Zero, negative, NaN, infinity, and rates too small to have a representable frame period are rejected by `build` and `run` through the `Result` they already return.

### Distribution

- 1.0 is distributed by git tag. Every package carries `publish = false` and states why it cannot produce a self-contained registry archive; crates.io stays open as a later decision rather than an implied promise.
- `dist.ps1` produces a versioned, per-platform archive with a SHA256 beside it, containing the engine, the reference extension, the ABI, a sample `bones.toml`, `LICENSE`, and generated third-party notices.

### Development

- `test.ps1` is the whole suite: fixtures, formatting, clippy with warnings denied, the default and all-feature test runs, and the documentation build.
- The web-enabled integration tests pass as a suite. They previously aborted when run together, because SDL treats the first thread to initialise it as its main thread and libtest gives every test a fresh one; the SDL-touching tests now share one owning thread.

## Before 1.0

The granular pre-release history — 176 commits from 2026-07-10 to 2026-08-09 — is archived at the tag `compat-main-2026-08-09`. The phase commits on `main` are squashed from it, and no version line existed before this release.
