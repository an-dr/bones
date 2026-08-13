# bones

A small native engine core — windows, tray icon, input (SDL), audio, logging, and a message bus — with all product behavior implemented as WASM extensions in any language.

Extensions talk to the core and to each other over one message bus. They never touch the OS and never render directly: they publish draw commands, widget specs, or web-panel JSON, and the engine presents. That boundary is what makes an extension hot-reloadable, sandboxed, and language-agnostic.

Two ways to use it:

- **Write extensions only.** Use the shipped engine binary as-is and put your game, tool, or GUI in WASM components. Most projects want this.
- **Embed the engine.** Depend on the crates, own the composition root, and inject your own native modules with `.module(...)`. For projects that need native capabilities the core does not ship.

**Status: 1.0.** The kernel, renderer, egui UI, audio, game-core, optional wry web panels, hot reload, orderly shutdown, extension flow-control budgets, and custom native-module injection all work today.

Two version lines, deliberately independent ([ADR-029](docs/adr/ADR-029-the-two-version-lines-are-the-two-public-surfaces.md)), both starting at 1.0.0:

| Line | What it covers | Moves when |
| --- | --- | --- |
| **engine** — `bones`, `bones-engine` | the Rust API an embedder links | the library surface changes |
| **ABI** — `bones:extension`, `bones-messages`, `bones-wasm-sdk` | the contract a `.wasm` extension is built against | the guest contract changes |

An extension outlives the engine build that loaded it, and may not be written in Rust; one number cannot promise both audiences. See [CHANGELOG.md](CHANGELOG.md) for what has moved.

## Getting it

**As a binary.** Download the release archive for your platform, or build one:

```sh
pwsh dist.ps1
```

That produces `dist/` plus a versioned, checksummed `bones-<version>-<os>-<arch>.zip` containing the engine, the reference extension, the ABI, a sample `bones.toml`, the licence, and third-party notices. Run `dist/bones(.exe)` directly, or without the bundling step:

```sh
cargo run -p bones
```

**As a library.** 1.0 is distributed by git tag, not through crates.io — every package carries `publish = false`, and each manifest says why it cannot produce a self-contained archive. [docs/structure.md](docs/structure.md#how-each-one-is-obtained) has the details and what a tag promises.

```toml
[dependencies]
bones-engine = { git = "https://github.com/an-dr/bones", tag = "v1.0.0" }
```

**As an extension author in Rust**, on the ABI line rather than the engine line:

```toml
[dependencies]
bones-wasm-sdk = { git = "https://github.com/an-dr/bones", tag = "abi-v1.0.0" }
```

Clone with `--recurse-submodules`; `vendor/pubsub-bus` is one.

## Building and testing

Requires a Rust toolchain (current stable — there is no MSRV policy), PowerShell 7+ (`pwsh`, cross-platform), and a C compiler with CMake — `crates/bones-engine/bones-kernel` builds SDL3 from source.

```sh
pwsh test.ps1
```

One command from a clean clone to a release-green tree: it builds the extension fixtures the integration tests need, then runs formatting, clippy with warnings denied, the default and all-feature test suites, and the documentation build.

## Platform support

| Platform | Status |
| --- | --- |
| Windows on ARM64 | **Supported.** Every release is built and tested here, web panels included |
| Windows on x64 | Expected to work; not routinely built |
| Linux, macOS | Best effort. The engine is written to be portable and the SDL and wry backends are cross-platform, but no release has been produced or tested on either. The wry web-panel integration tests are gated to Windows |

There is no CI ([docs/roadmap.md](docs/roadmap.md) tracks adding it), so "supported" means one maintainer's machine. Treat anything outside the first row as a report worth filing rather than a promise.

## Your first extension

Start at [crates/bones-extension-hello](crates/bones-extension-hello/README.md) — the reference extension, and the only one a distribution ships. It exercises the whole contract in [wit/extension.wit](wit/extension.wit): subscribing in `init`, handling `on-tick` and `on-message`, publishing, and cleaning up in `shutdown`.

Drop any built `.wasm` into `extensions/` next to wherever you run the engine and it loads on start.

For something richer, [examples/extensions/](examples/README.md) has ten runnable extensions — sprites, egui widgets, web panels, gamepad input, tilemaps, hot reload, persistence, and the watchdog — each proving one capability end to end, each with its own `build.ps1`. [examples/embedding/](examples/embedding/custom-engine/README.md) covers the other way in.

## Projects using bones

| Project | What it is |
| --- | --- |
| [commits](https://github.com/an-dr/commits) | A desktop Git client. Embeds bones and runs its Git Graph view in a wry web panel |
| [artificial-will-game-v2](https://github.com/an-dr/artificial-will-game-v2) | A game about a robot named Will. Embeds bones and ships its gameplay as extensions |
| [copper](https://github.com/an-dr-vibe/an-dr-copper) | A manifest-first automation host for AI-generated extensions. Embeds bones for its settings UI and tray |

## Documentation

Start at [docs/index.md](docs/index.md) — map of the architecture, detailed designs, decisions (ADRs), and worked examples. The short version:

- [docs/architecture.md](docs/architecture.md) — components, message flows, lifecycles.
- [docs/structure.md](docs/structure.md) — what lives where and what may depend on what.
- [docs/adr/](docs/adr/) — why the design is the way it is.

## Cutting a release

1. Decide which line moves. The engine line is `[workspace.package]`'s `version` in the root `Cargo.toml`; the ABI line is `bones:extension@` in [wit/extension.wit](wit/extension.wit) plus the explicit `version` in `bones-messages` and `bones-wasm-sdk`. They move independently — do not bump one to match the other.
2. If the ABI moved, regenerate the conformance vectors (`BONES_WRITE_VECTORS=1 cargo test --test conformance` from `crates/bones-messages`) and read the diff. It is the list of things you just broke.
3. `pwsh test.ps1` — all gates and both feature sets green.
4. Update [CHANGELOG.md](CHANGELOG.md) and commit.
5. `pwsh dist.ps1` on each platform you are publishing for. Keep each archive and its `.sha256`.
6. Tag: `v<version>` for the engine line, `abi-v<version>` for the ABI line. Tags are immutable — a fix is a new tag, never a moved one, because a git dependency has no checksum a consumer can verify against.
7. Attach the archives and their checksums to the release. Publish the checksum of the archive you actually uploaded: zip entries carry timestamps, so two runs of `dist.ps1` do not produce byte-identical archives.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for build prerequisites, how to run the tests, and the commit format.

## AI agents

The base agent policy — flows, roles, and skills — lives in [an-dr/agents](https://github.com/an-dr/agents). Install it globally for your AI tools; [AGENTS.md](AGENTS.md) holds the repo-specific rules that extend it.

## License

MIT — see [LICENSE](LICENSE).
