# wit

The WIT package defining the extension ABI — the one contract between the native core and every WASM extension, in any language (ADR-001).

[`core.wit`](core.wit) declares two halves:

- **`host-api`** — what an extension may call: `log`, `subscribe`, `publish`, `send`, `request-exit`, and display queries.
- **`world extension`** — what an extension must export: `init`, `shutdown`, `on-tick`, `on-message`.

## What belongs here

Only the interface definition. No implementation, no generated code: the host side is generated into `crates/bones-contract` by wasmtime's `bindgen!`, and the guest side by `wit_bindgen::generate!` in `crates/bones-wasm-sdk`.

Those two crates are the only places in the repository that name this directory. Extensions do not: a Rust extension depends on [`bones-wasm-sdk`](../crates/bones-wasm-sdk/README.md), which carries the package and generates the bindings for it.

Payload *encodings* do not belong here either. This package defines the shape of the calls; the typed payloads carried on core-defined topics live in [`bones-messages`](../crates/bones-messages/README.md), so that both the host and WASM guests can share one codec.

## Obtaining it

Extension authors outside this repository need `core.wit` to build against, whatever language they write in.

- **Rust** — do not fetch it at all. Depend on `bones-wasm-sdk`, which carries this package and generates the bindings.
- **Any other language** — take `wit/core.wit` from a distribution built by [`dist.ps1`](../dist.ps1), which copies it next to the engine binary it belongs to, or from this directory in the repository at the revision you are targeting.

Pin what you took. Record the `bones:core@<version>` line from the file's first line alongside your own sources, and re-check it when you update the engine.

## Changing it

This is a published ABI. Any change here breaks every extension built against the old package, including the three projects listed in the root [README](../README.md).

Enforcement is not advisory and not semver-ranged. wasmtime refuses to instantiate a component whose imported interface version differs from the host's at all, so a guest built against `bones:core@0.2.0` fails to load on a `@0.1.0` engine even when nothing else changed. The error names the mismatch directly:

```text
component imports instance bones:core/host-api@0.2.0, but a matching
implementation was not found in the linker
```

Structural changes are caught the same way even without a version bump, so a forgotten bump fails safe rather than silently mismatching. One exception is worth knowing: a component that imports nothing at all is version-agnostic, because unused imports are stripped before the check applies.

The practical consequence is that a bump costs every extension in existence, so prefer adding a topic and a typed message in `bones-messages` — that needs no ABI change at all.
