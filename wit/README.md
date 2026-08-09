# wit

The WIT package defining the extension ABI — the one contract between the
native core and every WASM extension, in any language (ADR-001).

[`core.wit`](core.wit) declares two halves:

- **`host-api`** — what an extension may call: `log`, `subscribe`, `publish`,
  `send`, `request-exit`, and display queries.
- **`world extension`** — what an extension must export: `init`, `shutdown`,
  `on-tick`, `on-message`.

## What belongs here

Only the interface definition. No implementation, no generated code: the host
side is generated into `core/contract` by wasmtime's `bindgen!`, and the guest
side by `wit_bindgen::generate!` in each extension.

Payload *encodings* do not belong here either. This package defines the shape
of the calls; the typed payloads carried on core-defined topics live in
[`shared/bones-messages`](../shared/bones-messages/README.md), so that both
the host and WASM guests can share one codec.

## Changing it

This is a published ABI. Any change here breaks every extension built against
the old package, including the three projects listed in the root
[README](../README.md). Prefer adding a topic and a typed message in
`bones-messages` — that needs no ABI change at all.

The package is versioned (`bones:core@0.1.0`); a breaking change should take
the version with it.
