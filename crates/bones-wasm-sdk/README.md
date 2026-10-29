# bones-wasm-sdk

Everything needed to write a bones extension in Rust, as one dependency.

An extension is a WASM guest: it never touches the OS, never renders, and reaches the engine only through the calls declared in [`wit/extension.wit`](../../wit/README.md). This crate carries that WIT package, generates the guest bindings from it, and re-exports the shared message vocabulary — so a guest declares one dependency instead of hand-copying `extension.wit` into its own tree and hardcoding a path to it.

## Public shape

- `Guest` — the trait an extension implements: `init`, `shutdown`, `on_tick`, `on_message`.
- `export!` — wires a `Guest` implementation up as the component's exports.
- `bindings` — the generated module, including `bones::extension::host_api` with `log`, `subscribe`, `publish`, `send`, `request_exit`, and the display queries.
- `messages` — `bones-messages`, re-exported: the typed payloads carried on core-defined topics.
- `bus` — typed calls over those raw imports: `publish(&message)`, `subscribe::<Message>()`, and one call per core endpoint (`load_state`, `save_state`, `read_file`, `web`). The topic comes from the message type, so a misspelt topic is a compile error rather than a message nobody receives.
- `Inbound` — every core topic an extension can receive, decoded from the `on_message` arguments in one call. Anything else, including an extension's own topics, arrives as `Inbound::Other`.
- `game_ui` — behind the `game-ui` feature: logical-canvas geometry, vertical menu layout, keyboard selection, and mouse hit-testing for in-world UI (ADR-025).

## Typed calls

The raw imports take a topic string and a byte payload. Use them for an extension's own topics; for core topics, the typed layer removes the two mistakes that compile and then fail silently, a wrong topic string and a wrong decoder:

```rust
use bones_wasm_sdk::bus;
use bones_wasm_sdk::messages::gfx::Clear;
use bones_wasm_sdk::messages::input::KeyDown;
use bones_wasm_sdk::messages::tick::Tick;
use bones_wasm_sdk::{Guest, Inbound};

struct Component;

impl Guest for Component {
    fn init() {
        bus::subscribe::<Tick>();
        bus::subscribe::<KeyDown>();
    }
    fn shutdown() {}
    fn on_tick(_dt: f32) {
        bus::publish(&Clear { r: 0, g: 0, b: 0, a: 255 });
    }
    fn on_message(topic: String, sender: String, payload: Vec<u8>) -> Option<Vec<u8>> {
        match Inbound::decode(&topic, &payload) {
            Ok(Inbound::KeyDown(key)) => log(Level::Info, &format!("{} from {sender}", key.key)),
            Ok(_) => {}
            Err(error) => log(Level::Warn, &format!("bad {topic} payload: {error}")),
        }
        None
    }
}

bones_wasm_sdk::export!(Component);
```

A payload that fails to decode is an `Err`, never silently skipped, so a version mismatch shows up in the log at the first message.

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
