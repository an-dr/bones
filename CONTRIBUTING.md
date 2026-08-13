# Contributing

Thanks for looking. bones is pre-release: interfaces still move, and there is no tagged version yet. Open an issue before starting anything large — the [roadmap](docs/roadmap.md) is short on purpose, and some things are deliberately deferred rather than missing.

## Building

You need:

- A Rust toolchain, plus the `wasm32-wasip2` target for extensions (`rustup target add wasm32-wasip2`).
- PowerShell 7+ (`pwsh`) — cross-platform, and the only script runtime used.
- A C compiler and CMake. `crates/bones-engine/bones-kernel` builds SDL3 from source; any compiler works (MSVC, clang, clang-cl, gcc), it is not MSVC-specific.

```sh
pwsh dist.ps1     # engine + reference extension into dist/
pwsh test.ps1     # build every fixture, then run all tests
```

Prefer those scripts over bare `cargo` for anything that touches `crates/bones-engine/bones-kernel`: they set up the compiler and cmake generator through [scripts/native-build-env.ps1](scripts/native-build-env.ps1). A bare `cargo build` on a machine with no generator configured fails inside SDL's cmake with a confusing error.

## Tests

`pwsh test.ps1` is the whole suite and must be green before you open a pull request. It builds the `.wasm` fixtures the integration tests load by path, runs `cargo test --workspace`, then runs the crates the root workspace excludes.

Some tests open a real SDL window and are serialized behind a lock; they are slower than the unit tests and live in their own binary for that reason.

## Code conventions

[docs/code-style.md](docs/code-style.md) is the precise pattern, and [AGENTS.md](AGENTS.md) states the underlying rules. In short: one type per file, tests in their own file, every crate has a README, and doc comments prefer short bullets to prose paragraphs.

These apply to new code and to code you substantially touch — they are not a mandate to churn files you are only passing through.

## Documentation

Docs capture **behavior and boundaries, not code**, and each layer has an altitude — the rules are at the top of [docs/index.md](docs/index.md). The test is: *an average refactoring must not require a documentation update.* If a change moves, splits, or renames code without changing observable behavior, no doc should need editing.

That is also why tutorials live in READMEs rather than under `docs/` — they are code-level by nature, and keeping them out preserves the altitude rule.

Architecture decisions go in [docs/adr/](docs/adr/), are immutable once recorded, and are superseded by new ADRs rather than edited. Use one for a lasting architectural decision, not a tactical or tooling choice.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/): a `type: imperative summary` subject of 72 characters or less, lowercase, no trailing period, optionally followed by a blank line and `- ` body entries explaining cause or motivation rather than restating the diff.

```text
fix: raise the minimum window width to 1200
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `style`, `chore`, `build`, `ci`, `perf`, `revert`. Needing two types means the commit does two things and should be split. Mark a breaking change with `!` and a `BREAKING CHANGE:` footer.

Commit messages record who is accountable for a change, so do not credit tooling in them — no assistant or generator trailers.
