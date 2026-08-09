# shared

Crates depended on by **both** the native host and WASM guest code. That dual
target is the entry requirement — it is why these are not in `core/`.

| Crate | What it is |
| --- | --- |
| [bones-messages](bones-messages/README.md) | Typed core messages and their payload codecs — tick, input, gfx, ui, audio, web, lifecycle, extension control |
| [game-ui](game-ui/README.md) | Optional theme-free menu layout and interaction for game extensions |

## Why they are outside the root workspace

Both must compile for `wasm32-wasip2` guests, so they cannot pull in wasmtime,
SDL, or anything else host-only. The root `Cargo.toml` excludes them from its
workspace to keep that honest, which is also why [test.ps1](../test.ps1) runs
their tests in a separate pass.

## What belongs here

A crate is a candidate only if a WASM extension and the native core both need
it. Host-only code goes in `core/`; extension-only code belongs in the
extension.

The practical case is a wire format: when the core defines a topic, the
encoder and decoder for its payload must be identical on both sides, and
`bones-messages` is where that agreement lives. The
[`wit/`](../wit/README.md) package defines the *calls*; these crates define
what travels through them.
