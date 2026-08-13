# bones-wasm-sdk

Everything needed to write a bones extension in Rust, as one dependency.

An extension is a WASM guest: it never touches the OS, never renders, and reaches the engine only through the calls declared in [`wit/extension.wit`](../../wit/README.md). This crate carries that WIT package, generates the guest bindings from it, and re-exports the shared message vocabulary — so a guest declares one dependency instead of hand-copying `extension.wit` into its own tree and hardcoding a path to it.

## Public shape

- `Guest` — the trait an extension implements: `init`, `shutdown`, `on_tick`, `on_message`.
- `export!` — wires a `Guest` implementation up as the component's exports.
- `bindings` — the generated module, including `bones::core::host_api` with `log`, `subscribe`, `publish`, `send`, `request_exit`, and the display queries.
- `messages` — `bones-messages`, re-exported: the typed payloads carried on core-defined topics.
- `game_ui` — behind the `game-ui` feature: logical-canvas geometry, vertical menu layout, keyboard selection, and mouse hit-testing for in-world UI (ADR-025).

## Constraints worth knowing

`export!` must be invoked in the `cdylib` crate that becomes the component. It cannot be a blanket impl, because the symbols it generates have to land in the final artifact.

The bindings are generated inside a `bindings` module rather than at the crate root. `pub_export_macro` marks the export macros `#[macro_export]`, which hoists them to the crate root, and a sibling `pub use` would then collide with them there. The module keeps the two apart; generating at the root fails with `E0255`.

This crate is on the **ABI version line**, together with `bones:extension` and `bones-messages`, and deliberately not on the engine's. It moves when the guest contract changes, never because the renderer did. A version mismatch is not advisory: wasmtime refuses to instantiate a component whose imported interface version differs from the host's, so an extension built against a bumped ABI stops loading rather than misbehaving.

Guests in other languages do not use this crate. They take `extension.wit` and the message wire format directly.

## Building

Extensions build for `wasm32-wasip2`:

```sh
cargo build --target wasm32-wasip2 --release
```

The `game_ui` module is pure geometry over `bones-messages`, so its tests run natively without a wasm target.
