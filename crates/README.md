# crates

Every crate in the repository. Folder name matches crate name throughout, so a path on disk and a name in `cargo tree` are the same string. The kernel infrastructure and optional modules nest under `bones-kernel/`; the launcher and pure versioning crate live beside it.

## The public surface

[bones-kernel](bones-kernel/README.md) is the one crate an embedder depends on. It holds the composition root — the builder API — directly, and re-exports a curated set of the crates nested beneath it. Everything under `bones-kernel/` that it does not re-export is an implementation detail — an embedder reaches all of it through this one crate, which is the single public library surface (ADR-030).

### Kernel infrastructure — always present, names no native module

| Crate | Responsibility |
| --- | --- |
| [bones-kernel-infra](bones-kernel/bones-kernel-infra/README.md) | Bus, logging, platform (SDL), persistence, and the frame loop — everything native modules depend *on*, plus everything that runs without naming one (ADR-030, ADR-031) |

The kernel infrastructure must build and run with no native modules registered at all.

### Native modules — optional, consumer-composed

Nested beside `bones-kernel-infra`, not inside it: each is its own crate, so a disabled feature means it never compiles, and Cargo — not discipline — stops one from depending on another (ADR-030). All individually optional; embedders add their own the same way (ADR-017).

| Crate | Responsibility |
| --- | --- |
| [bones-module-renderer](bones-kernel/bones-module-renderer/README.md) | Executes `gfx/*` draw batches, presents |
| [bones-module-ui](bones-kernel/bones-module-ui/README.md) | egui: widget specs in, draw data and events out |
| [bones-module-audio](bones-kernel/bones-module-audio/README.md) | `audio/*` music and effects, backed by kira |
| [bones-module-game-core](bones-kernel/bones-module-game-core/README.md) | ECS, collision, tilemaps, sprite animation |
| [bones-module-web](bones-kernel/bones-module-web/README.md) | wry panels and the bus/page JSON bridge |
| [bones-module-os](bones-kernel/bones-module-os/README.md) | Clipboard, browser, native file dialogs, and HTTPS fetch on a guest's behalf |
| [bones-module-single-instance](bones-kernel/bones-module-single-instance/README.md) | Single instance per user: the claim an application makes in `main`, and later launches carried onto the bus |
| [bones-module-desktop](bones-kernel/bones-module-desktop/README.md) | Desktop registration: called directly from an installer, or over `desktop/*` for one application the composition fixes |
| [bones-module-upgrade](bones-kernel/bones-module-upgrade/README.md) | Self-update over `upgrader/*`, with its own HTTPS download |
| [bones-module-devtools](bones-kernel/bones-module-devtools/README.md) | An opt-in, loopback-only channel that posts page messages and records the bus |
| [bones-module-wasm](bones-kernel/bones-module-wasm/README.md) | The opt-in extension host: loads `.wasm` components, gives each a bus endpoint, and owns quarantine, hot reload, and runtime activation (ADR-040) |

### Kernel libraries — plain Rust, no bus

| Crate | Responsibility |
| --- | --- |
| [bones-kernel-launch](bones-kernel/bones-kernel-launch/README.md) | Start a program in a folder, detached, or open a terminal there; the kernel's `launch` feature |
| [bones-kernel-watch](bones-kernel/bones-kernel-watch/README.md) | Watch folders and report debounced changes; the kernel's `watch` feature |

## Self-update

[bones-launcher](bones-launcher/README.md) selects and starts an installed version. It depends only on [bones-versioning](bones-versioning/README.md), which supplies pure version semantics. The optional upgrade module owns all downloading and installation; no standalone upgrading library remains (ADR-039).

## The one binary

[bones](bones/README.md) is the shipped engine executable and the default composition. It has **no access an embedder lacks** — enforced by the dependency graph, not by convention, since it depends on `bones-kernel` and nothing else here. [examples/embedding/custom-engine](../examples/embedding/custom-engine/README.md) writes the same one dependency.

## The shared contract

[bones-messages](bones-messages/README.md) is the one crate both the host and WASM guest code depend on — typed core messages and their payload codecs (tick, input, gfx, ui, audio, web, lifecycle, extension control). It is excluded from the root workspace because it must also compile for `wasm32-wasip2` guests, so it cannot pull in wasmtime, SDL, or anything else host-only; `cargo xtask test` runs its tests in a separate pass for the same reason. It cannot be folded into the guest SDK below without inverting the dependency graph — the host links it directly, and a package that runs `wit_bindgen::generate!` cannot sit underneath the engine.

## The guest SDK

[bones-wasm-sdk](bones-wasm-sdk/README.md) is the Rust extension SDK: the WIT package, its generated bindings, a re-export of `bones-messages`, and the optional `game-ui` toolkit (ADR-025). An extension author's one dependency. Like `bones-messages`, it is excluded from the root workspace and tested separately, for the same `wasm32-wasip2` reason.

## Rules

- Extensions depend on the ABI only — through `bones-wasm-sdk` in Rust — never on the host crates above.
- `bones-kernel-infra`'s bus and contract modules know nothing about presentation — messaging must stay usable headless.
- `bones-kernel-infra`'s logging module is a universal leaf: anyone may depend on the crate for it, and the crate itself depends on no other crate here.
- A native module never depends on another module's crate; it goes through a service in the registry `bones-kernel-infra`'s bus owns (ADR-031). `bones-kernel` is the sole exception, and only because composing them is its job.
- Nothing depends on `bones`, and nothing outside `bones-kernel/` depends on `bones-kernel-infra` or a `bones-module-*` crate except `bones-kernel` itself.

The dependency graph, and what counts as a violation, is in [docs/architecture/structure.md](../docs/architecture/structure.md). File-layout conventions — one type per file, tests out of line, what a crate README should say — are in [docs/code-style.md](../docs/code-style.md).
