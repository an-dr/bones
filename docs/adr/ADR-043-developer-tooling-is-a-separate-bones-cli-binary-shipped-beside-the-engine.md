# ADR-043: Developer tooling is a separate bones-cli binary shipped beside the engine

## Problem

A product built on the released engine needs tools the engine does not provide. It needs a project skeleton that builds on the first try. It needs one command that rebuilds an extension and hot-reloads it, a check that the machine and the project's pins match the engine, and a way for an AI agent to drive the running engine. Without them, every product rewrites the same build glue, and an agent spends its first turns getting it wrong.

`cargo xtask` already does this work inside this repository. It cannot leave it: it reads the repository's own layout and is built from source.

## Decision

- **The tools are one console program, `bones-cli`, with subcommands:** `new`, `doctor`, `build`, `dev`, and `mcp`.
- **It is a separate binary, not subcommands of `bones`.** `bones` stays a windowed application with one job: a release build has no console of its own, and an MCP server needs clean stdio.
- **Every release archive ships it beside `bones`.** It finds the engine as its sibling, so an archive is a complete toolkit with nothing else to install.
- **It depends on the public surfaces only:** `bones-messages` and the files in `wit/`. It never links the kernel, so it neither changes nor constrains what an embedder can rely on (ADR-030).
- **It is on the engine version line.** It builds projects against the tags of the release it came from.
- **What it shares with `cargo xtask` and `bones-test` lives in one crate, `bones-build`.** Building an extension for `wasm32-wasip2`, finding the `.wasm` in cargo's output, and checking that the target is installed are implemented there once. `bones-build` depends on no bones crate, so the engine's own automation, the product tools, and the test harness can all use it without new dependency edges. The rest of `cargo xtask` stays separate, because it builds and releases this repository rather than a product.

## Rationale

The engine executable and the tools have different audiences, lifetimes, and I/O. A player starts the engine from a shortcut. A developer or an agent runs the tools in a terminal or as a subprocess. Keeping them apart keeps the engine's surface small and lets the tools grow without touching the product's binary. Shipping them in the same archive costs one small executable, and it removes the question of where to get the right version.

## Rejected alternatives

- **Subcommands of `bones`.** The binary would mix a windowed application with stdio tools, and every tool would ship in every product built on the shipped engine.
- **A `cargo install` or `cargo generate` template.** It needs a Rust toolchain and network access before the first command, and it leaves the template's version free to drift from the engine's.
- **Grow `cargo xtask` and tell products to clone this repository.** A product should not need the engine's source to use the engine.
- **Let `cargo xtask` call `bones-cli` for the shared steps.** The engine's build would then depend on the product tool's command-line contract, and `bones-test` would need the whole binary for a few functions.
