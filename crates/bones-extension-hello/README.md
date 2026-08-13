# hello — the reference extension

The one extension a bones distribution ships, and the place to start if you are writing your own. It exercises the whole contract in [`wit/core.wit`](../../wit/core.wit): it subscribes in `init`, logs on every callback, publishes in response to messages, and cleans up in `shutdown`.

This page doubles as the write-your-first-extension tutorial.

## Build and run

```sh
pwsh build.ps1
```

Requires PowerShell 7+ (`pwsh`) — cross-platform, so this is the only build script needed on any OS. It runs `rustup target add wasm32-wasip2` (safe to repeat), builds the component, then builds the engine and assembles a runnable `dist/` beside this README: `dist/bones(.exe)`, `dist/bones.toml`, and `dist/crates/bones-extension-hello.wasm`.

Run `dist/bones(.exe)` and watch the log: `init`, then a `tick dt=...` line every frame.

**Always go through the script.** Plain `cargo build` does not error — it silently compiles a native `hello.dll`/`.so` instead, because nothing in the crate says which platform you meant. The output you want is `target/wasm32-wasip2/release/hello.wasm`, a WASM component loaded directly by `bones-kernel`'s wasm_extensions module with no separate componentization step.

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

The whole extension is [`src/lib.rs`](src/lib.rs). Generate the bindings from the WIT package, then implement `Guest`:

```rust
wit_bindgen::generate!({
    path: "../../wit",
    world: "extension",
});
```

**Subscribe in `init`.** Nothing is delivered to you until you ask for it:

```rust
fn init() {
    subscribe("core/tick");
    subscribe(CloseRequested::TOPIC);
    log(Level::Info, "init");
}
```

**Handle deliveries in `on_message`.** `topic` is empty for a direct send — those have a sender and a payload but no topic (ADR-010). Return `Some(bytes)` to reply to a direct send; return `None` for pub/sub deliveries, where the return value is ignored:

```rust
fn on_message(topic: String, sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
    log(Level::Debug, &format!("message on {topic} from {sender}"));
    publish("hello/received", topic.as_bytes());
    None
}
```

**Clean up in `shutdown`.** `hello` subscribes to the application close request and publishes from `shutdown`, which is what makes orderly cleanup visible when you close the demo window.

## Topics and payloads

`publish` and `subscribe` take a topic string; payloads are raw bytes. Two kinds of topic exist:

- **Core-defined topics** — `core/tick`, `input/*`, `gfx/*`, `ui/*`, `audio/*`, `web/*`, `core/lifecycle`. Their payloads are typed, and the encoders and decoders live in [`crates/bones-messages`](../../crates/bones-messages/README.md). Use them rather than hand-rolling a format — that is what `CloseRequested::TOPIC` above is. The full topic table is in [`docs/design/messaging.md`](../../docs/design/messaging.md).
- **Your own topics** — anything else, like `hello/received`. The engine treats these as opaque bytes and never inspects them; the extensions on both ends agree on the schema.

For a reply-shaped exchange rather than a broadcast, use `send` to a named endpoint instead of `publish`. It completes within the call (ADR-010).

## Writing your own

1. Copy this directory. In `Cargo.toml` keep `crate-type = ["cdylib"]` and the `[workspace]` line — an extension builds standalone, outside the root workspace.
2. Point `wit_bindgen::generate!` at the `wit/` directory and the `extension` world.
3. Subscribe to what you need in `init`, and implement the callbacks you care about — unused ones can stay empty.
4. Build with `build.ps1`, then drop the resulting `.wasm` into the `extensions/` directory beside the engine binary. It loads on start.

Any language with WASM Component Model support works the same way (ADR-001); Rust is only what these examples happen to use.

Watch out for two engine rules while developing:

- **A hung callback faults you, not the engine.** The watchdog traps a call that overruns its time budget and quarantines the extension (ADR-007); the engine and every other extension keep running. See [`examples/runaway_demo`](../../examples/runaway_demo/README.md).
- **Publishing has a budget.** Flooding the queue drops the excess, counts it, and faults you. See [`examples/flood_demo`](../../examples/flood_demo/README.md).

More runnable examples — sprites, widgets, web panels, input, tilemaps, hot reload, persistence — are in [`examples/`](../../examples/README.md).
