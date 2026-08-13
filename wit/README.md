# wit

The extension ABI — the one contract between the native core and every WASM extension, in any language (ADR-001). It has two halves, and an author outside Rust needs both.

[`core.wit`](core.wit) declares **the calls**:

- **`host-api`** — what an extension may call: `log`, `subscribe`, `publish`, `send`, `request-exit`, and display queries.
- **`world extension`** — what an extension must export: `init`, `shutdown`, `on-tick`, `on-message`.

[`wire-format.md`](wire-format.md) declares **the bytes those calls carry** on core-owned topics: the primitive encodings, the framing rules, and one section per topic, with machine-readable [conformance vectors](vectors/README.md) beside it.

The split is not arbitrary. WIT can describe a `list<u8>` payload but not what is inside it, so a `publish` signature alone tells a guest author nothing about how to build a `gfx/draw-sprite`. Both files carry the same `1.0.0` and move together.

## What belongs here

Only the contract. No implementation and no generated code: the host side is generated from `core.wit` by wasmtime's `bindgen!` inside [`bones-kernel`](../crates/bones-engine/bones-kernel/README.md), and the guest side by `wit_bindgen::generate!` in [`bones-wasm-sdk`](../crates/bones-wasm-sdk/README.md).

Those two crates are the only places in the repository that name this directory. Extensions do not: a Rust extension depends on `bones-wasm-sdk`, which carries the package and generates the bindings for it.

The Rust *implementation* of the wire format does not belong here either — it lives in [`bones-messages`](../crates/bones-messages/README.md), so the host and Rust guests share one codec. `wire-format.md` is the specification that crate implements, not a description of it; the vectors are what hold the two together.

## Obtaining it

Extension authors outside this repository need `core.wit` to build against, whatever language they write in.

- **Rust** — do not fetch it at all. Depend on `bones-wasm-sdk`, which carries this package and generates the bindings.
- **Any other language** — take this whole directory from a distribution built by [`dist.ps1`](../dist.ps1), which copies it next to the engine binary it belongs to, or from the repository at the revision you are targeting. You need `core.wit` to generate bindings, `wire-format.md` to build payloads, and `vectors/` to check that you built them correctly.

Pin what you took. Record the `bones:core@<version>` line from `core.wit`'s first line alongside your own sources, and re-check it when you update the engine.

## Known issue: the `shutdown` export name

A guest built for `wasm32-wasip2` links with a warning:

```text
rust-lld: function signature mismatch: shutdown
  >>> defined as (i32, i32) -> i32 in ...libstd...
  >>> defined as () -> void in ...your extension...
```

The cause is exact and is in this file. `world extension` exports a bare function named `shutdown`, and a world-level export's core WebAssembly symbol is its plain name — so it lands in the same flat namespace as POSIX `shutdown(sockfd, how)`, which `libstd` carries for socket support. The linker keeps both and the component's `shutdown` export is wired correctly, which is why extensions shut down properly and the integration tests pass.

What it is not safe against: a guest that actually calls socket shutdown through `std::net` may reach the wrong symbol. Nothing in this repository does, and an extension has no OS access to open a socket with in the first place — but a guest built with WASI socket support is not prevented from trying.

There is no fix that keeps the ABI. The export name **is** the contract, so renaming it, or moving the world's functions into an interface (which would qualify the symbol as `bones:core/<interface>#shutdown`), breaks every extension in existence. Both are on the table for the next ABI major; neither is worth a break on its own. Tracked in [docs/roadmap.md](../docs/roadmap.md).

## Changing it

This is a published ABI. Any change here breaks every extension built against the old package, including the three projects listed in the root [README](../README.md).

Enforcement is not advisory and not semver-ranged. wasmtime refuses to instantiate a component whose imported interface version differs from the host's at all, so a guest built against `bones:core@0.2.0` fails to load on a `@0.1.0` engine even when nothing else changed. The error names the mismatch directly:

```text
component imports instance bones:core/host-api@0.2.0, but a matching
implementation was not found in the linker
```

Structural changes are caught the same way even without a version bump, so a forgotten bump fails safe rather than silently mismatching. One exception is worth knowing: a component that imports nothing at all is version-agnostic, because unused imports are stripped before the check applies.

The practical consequence is that a bump costs every extension in existence, so prefer adding a topic and a typed message in `bones-messages` — that needs no ABI change at all.
