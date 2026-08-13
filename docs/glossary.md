# Glossary

The words this repository uses for its own parts. Most of them come from the WebAssembly Component Model (ADR-001) and mean there what they mean here; the rest name bones' own structure.

## The sandbox boundary

Almost every other term is positioned by this one distinction.

**Host** — the native program that embeds the WASM runtime. In bones that is the engine process: Rust, linking wasmtime. It *provides* the imports an extension may call and *invokes* the extension's exports. The kernel and native-module crates catalogued in [crates/](../crates/README.md) are host code; `bones-wasm-sdk` and `bones-extension-*` are guest code, below.

**Guest** — the WASM component the host loads. In bones that is an extension: a `.wasm` file the engine finds at startup. It *implements* the exports and *calls* the host's imports, and it can do nothing else — no OS access, no rendering, no linking native libraries.

[`wit/core.wit`](../wit/README.md) names both halves explicitly: `interface host-api` is what a guest may call, and `world extension` is what a guest must export. The host bindings are generated in `bones-kernel`'s contract module, the guest bindings in [`crates/bones-wasm-sdk`](../crates/bones-wasm-sdk/README.md), from that one file.

## Who is building what

**Extension** — a WASM guest that adds product behaviour. It is a build artifact, not a place in this repository: build it anywhere and drop the `.wasm` beside the engine binary. `crates/bones-extension-hello` is the reference one, and the only one a distribution ships.

**Embedder** — someone writing their *own* host: they depend on `bones-engine`, compose the modules they want, and produce their own binary. The three known embedders are listed in the root [README](../README.md). Note that embedder and extension author are roles one project plays at once, not two populations — all three write extensions too.

**Native module** — host-side, optional, and consumer-composed (ADR-011, ADR-017). It owns a native resource — a GPU surface, an egui context, a webview — or a simulation the engine runs, and it joins the frame loop by implementing the `Module` trait and being injected with `.module(...)`. `renderer`, `ui`, `audio`, `game-core`, and `web` are the shipped ones.

**Kernel tier** — the host code that is always present and names no native module, as opposed to the modules themselves: `bus`, `wasm-extensions` (including extension loading and supervision), `contract`, `platform`, `logging`, and `runner`, merged into the one `bones-kernel` crate (ADR-030, ADR-031). What stays outside it is the composition root — the builder in `bones-engine`, the one place that names concrete module types. The kernel must build and run with no modules registered at all. See [crates/](../crates/README.md), which lists both tiers.

**Guest library** — a Rust crate compiled *into* a `.wasm`, not linked by the engine. It owns no native resource and is pure computation over messages. `bones-wasm-sdk`'s `game_ui` module is the one bones ships (ADR-025).

**Service** — a typed value one module publishes into the registry for others to resolve, so a consumer depends on `bus` rather than on the provider's crate. `window-surface`, `bus`, and `draw-target` are the shipped ones, and they are the only way one native module reaches another (ADR-031); see [design/modules.md](design/modules.md).

## The two public surfaces

Each has its own version line, and the line is what tells you which surface you are on.

**Engine surface** — `bones-engine`, what an embedder pins. Moves in lockstep with the kernel and native-module crates in `crates/`.

**ABI surface** — `bones:core` in `wit/core.wit` plus `bones-messages`, what an extension author pins, in any language. Moves only when the guest contract changes. `bones-wasm-sdk` rides this line, since it is a packaging of exactly that contract for Rust.

## Where does my code go

| What you are writing | Where it goes | Compiles for |
| --- | --- | --- |
| A native module — owns a resource, injected with `.module(...)` | `crates/bones-engine/`, named `bones-module-<name>` | native |
| Always-present host code — bus, logging, platform, the frame loop | `crates/bones-engine/bones-kernel/`, as a module of that crate | native |
| A WASM extension | anywhere; `crates/bones-extension-<name>` only if a fresh install must ship it, `examples/` if it demonstrates one capability | `wasm32-wasip2` |
| A library both host and guest link | `crates/bones-messages` — the one crate that meets this bar | both |
| A Rust guest library | `crates/bones-wasm-sdk`, as a feature-gated module | `wasm32-wasip2` |

Two rules decide it, and the second one only if the first is ambiguous. Which side of the sandbox does this run on? And for a library, how many sides need it?

The dependency graph enforces the answer. An extension depends only on `bones-wasm-sdk` and never on a host crate, so if your new code wants to reach `bones_bus::`, it is host code and belongs in `crates/`, named `bones-<name>`. If it cannot, it is guest code.
