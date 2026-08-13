# crates

Every crate in the repository. Folder name matches crate name throughout, so a path on disk and a name in `cargo tree` are the same string. Five crates sit at this top level; six more nest under `bones-engine/` itself (ADR-030).

## The public surface

[bones-engine](bones-engine/README.md) is the one crate an embedder depends on. It holds the composition root — the builder API — directly, and re-exports a curated set of the crates nested beneath it. Everything under `bones-engine/` that it does not re-export is an implementation detail — an embedder reaches all of it through this one crate, which is the single public library surface (ADR-030).

### Kernel — always present, names no native module

| Crate | Responsibility |
| --- | --- |
| [bones-kernel](bones-engine/bones-kernel/README.md) | Bus, logging, host-side WIT bindings, platform (SDL), the frame loop, and WASM extension loading and supervision — everything native modules depend *on*, plus everything that runs without naming one (ADR-030, ADR-031) |

The kernel must build and run with no native modules registered at all.

### Native modules — optional, consumer-composed

Nested beside `bones-kernel`, not inside it: each is its own crate, so a disabled feature means it never compiles, and Cargo — not discipline — stops one from depending on another (ADR-030). All individually optional; embedders add their own the same way (ADR-017).

| Crate | Responsibility |
| --- | --- |
| [bones-module-renderer](bones-engine/bones-module-renderer/README.md) | Executes `gfx/*` draw batches, presents |
| [bones-module-ui](bones-engine/bones-module-ui/README.md) | egui: widget specs in, draw data and events out |
| [bones-module-audio](bones-engine/bones-module-audio/README.md) | `audio/*` music and effects, backed by kira |
| [bones-module-game-core](bones-engine/bones-module-game-core/README.md) | ECS, collision, tilemaps, sprite animation |
| [bones-module-web](bones-engine/bones-module-web/README.md) | wry panels and the bus/page JSON bridge |

## The one binary

[bones](bones/README.md) is the shipped engine executable and the default composition. It has **no access an embedder lacks** — enforced by the dependency graph, not by convention, since it depends on `bones-engine` and nothing else here. [examples/embedding/custom-engine](../examples/embedding/custom-engine/README.md) writes the same one dependency.

## The shared contract

[bones-messages](bones-messages/README.md) is the one crate both the host and WASM guest code depend on — typed core messages and their payload codecs (tick, input, gfx, ui, audio, web, lifecycle, extension control). It is excluded from the root workspace because it must also compile for `wasm32-wasip2` guests, so it cannot pull in wasmtime, SDL, or anything else host-only; [test.ps1](../test.ps1) runs its tests in a separate pass for the same reason. It cannot be folded into the guest SDK below without inverting the dependency graph — the host links it directly, and a package that runs `wit_bindgen::generate!` cannot sit underneath the engine.

## The guest SDK

[bones-wasm-sdk](bones-wasm-sdk/README.md) is the Rust extension SDK: the WIT package, its generated bindings, a re-export of `bones-messages`, and the optional `game-ui` toolkit (ADR-025). An extension author's one dependency. Like `bones-messages`, it is excluded from the root workspace and tested separately, for the same `wasm32-wasip2` reason.

## Shipped extensions

A crate prefixed `bones-extension-` is an actual extension a bones distribution ships, as opposed to a demo — that prefix is reserved for exactly this (ADR-030). Exactly one exists:

- [bones-extension-hello](bones-extension-hello/README.md) — the reference extension, and the write-your-first-extension tutorial.

[dist.ps1](../dist.ps1) builds every `bones-extension-*` crate into `dist/extensions/`, which is why the bar for adding one is high: a distribution is the engine plus its reference extension, not a demo bundle. Anything that exists to demonstrate a capability belongs in [examples/extensions/](../examples/README.md) instead, where its name ends in `-demo`. You do not need to add a crate here to write your own extension: build it anywhere and drop the `.wasm` into the directory `bones.toml`'s `extensions_dir` names; it loads on start.

## Rules

- Extensions depend on the ABI only — through `bones-wasm-sdk` in Rust — never on the host crates above.
- `bones-kernel`'s bus and contract modules know nothing about presentation — messaging must stay usable headless.
- `bones-kernel`'s logging module is a universal leaf: anyone may depend on the crate for it, and the crate itself depends on no other crate here.
- A native module never depends on another module's crate; it goes through a service in the registry `bones-kernel`'s bus owns (ADR-031). `bones-engine` is the sole exception, and only because composing them is its job.
- Nothing depends on `bones`, and nothing outside `bones-engine/` depends on `bones-kernel` or a `bones-module-*` crate except `bones-engine` itself.

The dependency graph, and what counts as a violation, is in [docs/structure.md](../docs/structure.md). File-layout conventions — one type per file, tests out of line, what a crate README should say — are in [docs/code-style.md](../docs/code-style.md).
