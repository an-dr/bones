# embedding-demo — the embedding guide

A parent project injecting its own native module and building its own engine
binary, with no privileged access to bones internals
([docs/design/modules.md](../../docs/design/modules.md), ADR-017).

This page doubles as the guide to embedding bones as a library.

## When to embed

Most projects should not. If your product behavior fits in WASM extensions,
use the shipped engine binary and start at
[extensions/hello](../../extensions/hello/README.md) — you get sandboxing,
hot reload, and language choice for free.

Embed when you need something the engine cannot ship for you:

- **A native capability with no bus vocabulary yet** — clipboard, native file
  dialogs, a database driver, a hardware SDK.
- **Trusted, in-process work** — anything that must not pay the WASM boundary
  cost or must hold a native handle across frames.
- **Your own composition** — a different set of modules, or no window at all.

The cost is that you own the composition root and the update burden: your
`Cargo.toml` pins bones crates by path or git, and their interfaces are still
moving.

## Run it

```sh
cargo run
```

Requires a C toolchain and CMake — `platform` builds SDL3 from source. If
cmake cannot find a generator and Ninja is installed, set
`CMAKE_GENERATOR=Ninja`; the repository's
[scripts/native-build-env.ps1](../../scripts/native-build-env.ps1) does
exactly this and is what `dist.ps1` and `test.ps1` use.

## What an embedder actually depends on

A separate `[workspace]`, depending on bones crates by relative path the same
way a real embedder would depend on them as a git subrepo or path dependency:

```toml
[dependencies]
runner = { path = "../../core/runner" }
bus    = { path = "../../core/bus" }
logging = { path = "../../core/logging" }
```

Nothing here uses anything an external embedder could not. That is the point
of this crate existing: `app`, the shipped engine binary, has no access you
lack — it uses the same public builder API
([docs/structure.md](../../docs/structure.md)).

## Writing a native module

A native module is a plain Rust type implementing two traits from `bus`. On
the bus it is indistinguishable from a WASM extension (ADR-011) — the same
topics, the same delivery rules — it simply runs natively and in-process.

`Handler` receives bus deliveries:

```rust
impl Handler for Clock {
    fn handle(&mut self, _envelope: &Envelope) { /* ... */ }
}
```

`Module` names it and gets one `init` to declare subscriptions, exactly like
an extension's `init`:

```rust
impl Module for Clock {
    fn name(&self) -> &str { "clock" }

    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe("core/tick");
        Ok(())
    }
}
```

## The composition root

`main.rs` is the whole composition — build an `Engine`, inject modules, run:

```rust
runner::Engine::new()
    .logger(logger.clone())
    .module(Clock::new(logger))
    .run()
```

This one runs **headless**: no window, no renderer, no extensions. That is
the kernel's baseline, and every native module is optional — the engine must
build and run with zero of them registered.

Add what you need on top:

| Builder call | Gives you |
| --- | --- |
| `.window(title, w, h)` | An SDL window, input, and the frame loop |
| `.renderer()` | `gfx/*` draw commands executed against that window |
| `.module(...)` | Any module, first-party or your own |
| `.logger(...)` | Your own sink, so engine logs land in your stream |
| `.saves_dir(...)` | Where extension persistence writes |

The first-party modules — renderer, ui, audio, game-core, web — go in through
the same `.module(...)` door your own does, and the native presentation stack
is feature-gated so a headless build carries no SDL or wry dependency
(ADR-027). A presentation can also attach to a live headless engine and
release its window when closed (ADR-028).

For the frame phases modules hook, the typed service registry they use to
find each other, and the rules on what may depend on what, see
[docs/design/modules.md](../../docs/design/modules.md) and
[docs/structure.md](../../docs/structure.md).
