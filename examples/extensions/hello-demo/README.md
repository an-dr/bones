# hello-demo

The smallest complete extension, and the place to start if you are writing your own. It exercises the whole contract in [`wit/extension.wit`](../../../wit/extension.wit): it subscribes in `init`, logs on every callback, publishes in response to messages, and cleans up in `shutdown`.

This page doubles as the write-your-first-extension tutorial.

## Build and run

```sh
cargo xtask example hello-demo
```

Run it from the repository root. It checks that the `wasm32-wasip2` target is installed (and says how to add it if not), builds the component, then builds the engine and assembles a runnable `dist/` beside this README: `dist/bones(.exe)`, `dist/bones.toml`, and `dist/extensions/hello_demo.wasm`.

Run `dist/bones(.exe)` and watch the log: `init`, then a `tick dt=...` line every frame.

**Always pass the target.** To build only the component, run `cargo build --target wasm32-wasip2 --release` in this directory. Plain `cargo build` does not error — it silently compiles a native `hello_demo.dll`/`.so` instead, because nothing in the crate says which platform you meant. The output you want is `target/wasm32-wasip2/release/hello_demo.wasm`, a WASM component loaded directly by `bones-kernel-infra`'s wasm_extensions module with no separate componentization step.

## The four exports

An extension is a WASM component implementing the `extension` world. The engine calls you; you never own a thread or a loop.

| Export | Called | Use it for |
| --- | --- | --- |
| `init` | Once, at load | Subscribing to topics, restoring saved state |
| `on-tick(dt)` | Every frame, **only if** you subscribed to `core/tick` | Simulation, immediate-mode UI |
| `on-message(topic, sender, payload)` | On every delivery you subscribed to, and on every direct send | Reacting to input, other extensions, engine events |
| `shutdown` | Once, before unload | Cleanup, persisting state |

An extension that subscribes to nothing and skips `core/tick` costs nothing per frame. Subscribe to `core/tick` only if you actually need a frame pulse.

## Walking through the source

The whole extension is [`src/lib.rs`](src/lib.rs). It depends on `bones-wasm-sdk` alone, implements `Guest`, and hands the type to `export!`:

```rust
use bones_wasm_sdk::{bus, Guest};

struct Component;

impl Guest for Component { /* init, shutdown, on_tick, on_message */ }

bones_wasm_sdk::export!(Component);
```

**Subscribe in `init`.** Nothing is delivered to you until you ask for it. `bus::subscribe` takes the topic from the message type, so it cannot be misspelt:

```rust
fn init() {
    bus::subscribe::<Tick>();
    bus::subscribe::<CloseRequested>();
    log(Level::Info, "init");
}
```

**Handle deliveries in `on_message`.** `topic` is empty for a direct send — those have a sender and a payload but no topic (ADR-010). Return `Some(bytes)` to reply to a direct send; return `None` for pub/sub deliveries, where the return value is ignored. To read a core message, decode the arguments with `Inbound::decode(&topic, &payload)` and match on the variant; `hello` only logs the topic, so it needs no decoding:

```rust
fn on_message(topic: String, sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
    log(Level::Debug, &format!("message on {topic} from {sender}"));
    publish("hello/received", topic.as_bytes());
    None
}
```

**Clean up in `shutdown`.** `hello` subscribes to the application close request and publishes from `shutdown`, which is what makes orderly cleanup visible when you close the demo window.

## Topics and payloads

The raw `publish` and `subscribe` imports take a topic string; payloads are raw bytes. Two kinds of topic exist:

- **Core-defined topics** — `core/tick`, `input/*`, `gfx/*`, `ui/*`, `audio/*`, `web/*`, `core/lifecycle`, and the rest in [`wit/wire-format.md`](../../../wit/wire-format.md). Their payloads are typed, and the encoders and decoders live in [`crates/bones-messages`](../../../crates/bones-messages/README.md). Use the typed calls — `bus::publish(&message)`, `bus::subscribe::<Message>()`, and `Inbound::decode` — rather than hand-rolling a format. The full topic table is in [`docs/design/messaging.md`](../../../docs/design/messaging.md).
- **Your own topics** — anything else, like `hello/received`. The engine treats these as opaque bytes and never inspects them; the extensions on both ends agree on the schema.

For a reply-shaped exchange rather than a broadcast, use `send` to a named endpoint instead of `publish`. It completes within the call (ADR-010).

## Writing your own

1. Copy this directory. In `Cargo.toml` keep `crate-type = ["cdylib"]` and the `[workspace]` line — an extension builds standalone, outside the root workspace.
2. Point `wit_bindgen::generate!` at the `wit/` directory and the `extension` world.
3. Subscribe to what you need in `init`, and implement the callbacks you care about — unused ones can stay empty.
4. Build with `cargo build --target wasm32-wasip2 --release`, then drop the resulting `.wasm` into the `extensions/` directory beside the engine binary. It loads on start.

Any language with WASM Component Model support works the same way (ADR-001); Rust is only what these examples happen to use.

Watch out for two engine rules while developing:

- **A hung callback faults you, not the engine.** The watchdog traps a call that overruns its time budget and quarantines the extension (ADR-007); the engine and every other extension keep running. See [`examples/extensions/runaway-demo`](../../extensions/runaway-demo/README.md).
- **Publishing has a budget.** Flooding the queue drops the excess, counts it, and faults you. See [`examples/extensions/flood-demo`](../../extensions/flood-demo/README.md).

More runnable examples — sprites, widgets, web panels, input, tilemaps, hot reload, persistence — are in [`examples/`](../../README.md).
