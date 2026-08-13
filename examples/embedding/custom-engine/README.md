# custom-engine — the embedding guide

Your own `bones` executable: the shipped engine, plus one native capability, plus a message vocabulary bones does not own.

This page doubles as the guide to embedding bones as a library.

## When to embed

Most projects should not. If your product behaviour fits in WASM extensions, use the shipped engine binary and start at [crates/bones-extension-hello](../../../crates/bones-extension-hello/README.md) — you get sandboxing, hot reload, and language choice for free.

Embed when the capability you need does not exist and cannot, because it requires native access the sandbox refuses to grant:

- **An OS capability with no bus vocabulary yet** — clipboard, native file dialogs, a database driver, a hardware SDK.
- **Trusted, in-process work** — anything that must not pay the WASM boundary cost, or must hold a native handle across frames.
- **Your own composition** — a different set of modules, or no window at all.

The cost is that you own the composition root and the update burden: your `Cargo.toml` pins bones by git tag, and you move when it moves.

## What this example is

Three crates, because that is what the pattern actually requires:

| Crate | Target | Role |
| --- | --- | --- |
| [messages](messages/src/lib.rs) | both | The vocabulary: topics and payloads bones does not define |
| [engine](engine/src/main.rs) | native | The binary — the bones stack plus one module |
| [extension](extension/src/lib.rs) | `wasm32-wasip2` | A guest that speaks that vocabulary |

The native module answers questions a sandboxed extension cannot answer for itself — hostname, working directory, an environment variable — using `std` alone. That is the smallest honest illustration of why anyone embeds: not that the capability is hard, but that the sandbox correctly refuses it.

## Run it

```sh
pwsh build.ps1
```

Builds all three, plus the stock `hello` extension, into `dist/`. Run `dist/custom-engine(.exe)`. Requires a C toolchain and CMake — the engine links SDL3, built from source. If cmake cannot find a generator and Ninja is installed, set `CMAKE_GENERATOR=Ninja`; [scripts/native-build-env.ps1](../../../scripts/native-build-env.ps1) does exactly this and is what `dist.ps1` and `test.ps1` use.

The log shows `host-probe` asking the native module for each fact, and a widget panel shows two of them. `hello` runs beside it untouched — a stock extension and a custom one in one process, against one engine.

## The difference from shipped bones

Put [engine/src/main.rs](engine/src/main.rs) next to [crates/bones/src/main.rs](../../../crates/bones/src/main.rs). The composition is the same — extensions directory, window, renderer, ui — with one line added:

```rust
.module(HostFacts::new(logger))
```

That line is the whole of embedding. Everything else on this page exists to make that line useful.

## Defining your own messages

The bus is byte-oriented and open ([ADR-016](../../../docs/adr/ADR-016-typed-core-messages.md)): bones defines the wire contract for topics *it* owns, and treats everything else as opaque bytes it neither inspects nor constrains. So a vocabulary is yours to define, with no release to wait for and no topic to request.

Two properties of the engine make one crate serve both sides of the sandbox, and both are deliberate:

- **`bones-messages` has no dependencies**, so it compiles for the native host and for `wasm32-wasip2`. A crate that depends only on it inherits that.
- **The codec is public.** `Reader` and `Writer` are the same primitives the core messages use, so a custom payload is encoded exactly like a bones-owned one — same little-endian layout, same framing, same failure modes ([wit/wire-format.md](../../../wit/wire-format.md)).

A guest written in another language would implement the same two types from that document. Nothing here is Rust-only except the convenience of sharing a crate.

Treat your vocabulary as the contract it is. Once an extension ships against it, changing the bytes breaks that extension exactly as an ABI change would — which is why [messages/src/tests.rs](messages/src/tests.rs) round-trips every variant, and why [test.ps1](../../../test.ps1) runs it.

## Request/reply, not publish

A question with an answer is a direct `send` ([ADR-010](../../../docs/adr/ADR-010-synchronous-send.md)) to a named endpoint, and it completes inside the caller's own call:

```rust
let reply = send(ENDPOINT, &FactsRequest { fact }.encode());
```

The native module implements `Module::respond` to answer — the same mechanism `persistence` and `files` use, and the same one any extension can be on the receiving end of. Publishing would be the wrong shape here: the caller wants an answer, not a broadcast.

Note what happens when the contract is not met. Drop `host_probe.wasm` into the *shipped* `bones` binary and the send fails with an unknown endpoint, because the module answering it does not exist there. The extension handles that plainly, because it is the failure an embedder's users will actually hit.

## Writing a native module

A native module is a plain Rust type implementing two traits from `bones_engine::bus`. On the bus it is indistinguishable from a WASM extension ([ADR-011](../../../docs/adr/ADR-011-native-core-modules.md)) — same topics, same delivery rules — it simply runs natively and in-process.

- `Handler::handle` receives bus deliveries for whatever `init` subscribed to.
- `Module::init` requests subscriptions and resolves services.
- `Module::respond` answers direct sends addressed to it by name.
- `filter_event`, `render`, `present`, and `shutdown` are the remaining hooks, each defaulting to a no-op.

[engine/src/host_facts.rs](engine/src/host_facts.rs) uses three of them and ignores the rest. For the frame phases, the typed service registry modules use to reach each other, and the rules on what may depend on what, see [docs/design/modules.md](../../../docs/design/modules.md) and [docs/structure.md](../../../docs/structure.md).
