# Glossary

The words this repository uses for its own parts. Most of them come from the WebAssembly Component Model (ADR-001) and mean there what they mean here; the rest name bones' own structure.

## The sandbox boundary

Almost every other term is positioned by this one distinction.

**Host** — the native program that embeds the WASM runtime. In bones that is the engine process: Rust, linking wasmtime. It *provides* the imports an extension may call and *invokes* the extension's exports. The kernel infrastructure and native-module crates catalogued in [crates/](../crates/README.md) are host code; `bones-wasm-sdk` and `bones-extension-*` are guest code, below.

**Guest** — the WASM component the host loads. In bones that is an extension: a `.wasm` file the engine finds at startup. It *implements* the exports and *calls* the host's imports, and it can do nothing else — no OS access, no rendering, no linking native libraries.

[`wit/extension.wit`](../wit/README.md) names both halves explicitly: `interface host-api` is what a guest may call, and `world extension` is what a guest must export. The host bindings are generated in `bones-kernel-infra`'s contract module, the guest bindings in [`crates/bones-wasm-sdk`](../crates/bones-wasm-sdk/README.md), from that one file.

## Who is building what

**Extension** — a WASM guest that adds product behaviour, hosted by the opt-in `wasm` module (ADR-040). It is a build artifact, not a place in this repository: build it anywhere and drop the `.wasm` beside the engine binary. [`examples/extensions/hello-demo`](../examples/extensions/hello-demo/README.md) is the place to start; a distribution ships none.

**Embedder** — someone writing their *own* host: they depend on `bones-kernel`, compose the modules they want, and produce their own binary. The three known embedders are listed in the root [README](../README.md). Note that embedder and extension author are roles one project plays at once, not two populations — all three write extensions too.

**Native module** — host-side, optional, and consumer-composed (ADR-011, ADR-017). It owns a native resource — a GPU surface, an egui context, a webview — or a simulation the engine runs, and it joins the frame loop by implementing the `Module` trait and being injected with `.module(...)`. `renderer`, `ui`, `audio`, `game-core`, `web`, and `os` are the shipped ones.

**Kernel** — the assembled, running system an embedder builds with `bones_kernel::Kernel`: the kernel infrastructure, the native modules composed into it, and, when the `wasm` module is one of them, the extensions it hosts (ADR-038, ADR-040). Not to be confused with the product, which is described as an engine.

**Kernel infrastructure tier** — the host code that is always present and names no native module, as opposed to the modules themselves: `bus`, `platform`, `logging`, `persistence`, and `runner`, merged into the one `bones-kernel-infra` crate (ADR-030, ADR-031). Extension loading and supervision belong to the opt-in `wasm` module (ADR-040). What stays outside it is the composition root — the builder in `bones-kernel`, the one place that names concrete module types. The kernel infrastructure must build and run with no modules registered at all. See [crates/](../crates/README.md), which lists both tiers.

**Guest library** — a Rust crate compiled *into* a `.wasm`, not linked by the engine. It owns no native resource and is pure computation over messages. `bones-wasm-sdk`'s `game_ui` module is the one bones ships (ADR-025).

**Service** — a typed value one module publishes into the registry for others to resolve, so a consumer depends on `bus` rather than on the provider's crate. `window-surface`, `bus`, and `draw-target` are the shipped ones, and they are the only way one native module reaches another (ADR-031); see [design/modules.md](design/modules.md).

## The two public surfaces

Each has its own version line, and the line is what tells you which surface you are on.

**Kernel surface** — `bones-kernel`, what an embedder pins. Moves in lockstep with the kernel infrastructure and native-module crates in `crates/`.

**ABI surface** — `bones:extension` in `wit/extension.wit` plus `bones-messages`, what an extension author pins, in any language. Moves only when the guest contract changes. `bones-wasm-sdk` rides this line, since it is a packaging of exactly that contract for Rust.

## Where does my code go

| What you are writing | Where it goes | Compiles for |
| --- | --- | --- |
| A native module — owns a resource, injected with `.module(...)` | `crates/bones-kernel/`, named `bones-module-<name>` | native |
| Always-present host code — bus, logging, platform, the frame loop | `crates/bones-kernel/bones-kernel-infra/`, as a module of that crate | native |
| A WASM extension | anywhere; `crates/bones-extension-<name>` only if a fresh install must ship it, `examples/` if it demonstrates one capability | `wasm32-wasip2` |
| A library both host and guest link | `crates/bones-messages` — the one crate that meets this bar | both |
| A Rust guest library | `crates/bones-wasm-sdk`, as a feature-gated module | `wasm32-wasip2` |

Two rules decide it, and the second one only if the first is ambiguous. Which side of the sandbox does this run on? And for a library, how many sides need it?

The dependency graph enforces the answer. An extension depends only on `bones-wasm-sdk` and never on a host crate, so if your new code wants to reach `bones_bus::`, it is host code and belongs in `crates/`, named `bones-<name>`. If it cannot, it is guest code.
