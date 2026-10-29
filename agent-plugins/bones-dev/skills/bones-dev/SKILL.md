---
name: bones-dev
description: Change the bones engine itself — its crates, the wire format, the extension host, the developer tools, or the release. Use in the bones repository (a Cargo workspace with crates/bones-kernel, wit/wire-format.json, and xtask/), not in a product built on bones; for that, use the bones skill.
---

# bones-dev

bones is a native core with a message bus and optional sandboxed WebAssembly extensions. This skill is for working on the engine. The repository's `AGENTS.md` holds its conventions and commit scopes, and [docs/index.md](../../../../docs/index.md) maps the documentation. The documentation is the authority: a design changes in `docs/` before its code does.

## Find your way

| Path | Holds |
| --- | --- |
| `crates/bones-kernel/` | the embedder's one crate, its modules (`bones-module-*`), and `bones-kernel-infra` (bus, runner, platform, logging) |
| `crates/bones-messages/`, `crates/bones-wasm-sdk/` | the ABI line: the message codec and the guest SDK, outside the root workspace so they build for `wasm32-wasip2` |
| `crates/bones/` | the shipped engine executable |
| `crates/bones-cli/`, `crates/bones-test/`, `crates/bones-build/` | the developer tools, the test harness, and the extension build they share (ADR-043) |
| `wit/` | the extension ABI: `extension.wit`, `wire-format.json`, and the conformance vectors |
| `xtask/` | every build, gate, and generator, as `cargo xtask <command>` |
| `examples/`, `tests/agent/` | runnable examples, and the agent benchmark |
| `agent-plugins/` | everything AI-specific, as Agent Plugins packages: `bones-user` and this one |

## Check a change

- `cargo xtask doctor` names a missing prerequisite and how to install it. SDL builds from source, so cmake and a C compiler are needed.
- Run the tests of what you touched while working, then `cargo xtask test` once at the end: formatting, clippy with warnings denied, both feature sets, the kernel without `wasm`, the standalone crates, and the rustdoc build ([docs/contributing/testing.md](../../../../docs/contributing/testing.md)).
- Generated files are checked for staleness by tests. After changing a source, run its generator and commit both: `wire-format`, `vectors`, `ts-codec`, and `plugin-docs`. `cargo xtask plugins` checks the agent plugins.

## Change a message

The steps are in [references/wire-format.md](references/wire-format.md). In short: specify it in `wit/wire-format.json`, implement it in `bones-messages` with a conformance vector, then regenerate. Whether it moves the ABI line decides everything else.

## Keep the invariants

bones is built for AI-assisted product development (ADR-044). Read [references/invariants.md](references/invariants.md) before adding a topic, a failure path, a capability, or a tool, and say in your summary how the change keeps each invariant it touches.

## Traps

- **SDL's main thread.** SDL owns the thread that first initialised it. A repository test that opens a window uses `sdl_test!`, never a bare `#[test]` ([testing.md](../../../../docs/contributing/testing.md)). `bones-test`'s rendered Apps take turns under one lock and quit SDL between them instead, because product tests cannot use the macro.
- **Version lines.** The engine line and the ABI line move independently ([docs/contributing/releasing.md](../../../../docs/contributing/releasing.md)). A changed byte in an existing message breaks every extension; prefer a new topic.
- **ADRs.** An integrated ADR is immutable and is superseded by a new one. An ADR written on the current branch is a draft, and is edited in place.
- **Formatting.** Format per package. `cargo fmt --all` would also rewrite the vendored `pubsub-bus` submodule.
