# Contributing

Thanks for looking. bones is pre-release: interfaces still move, and there is no tagged version yet. Open an issue before starting anything large — the [roadmap](docs/roadmap.md) is short on purpose, and some things are deliberately deferred rather than missing.

## Building

You need:

- A Rust toolchain, plus the `wasm32-wasip2` target for extensions (`rustup target add wasm32-wasip2`).
- PowerShell 7+ (`pwsh`) — cross-platform, and the only script runtime used.
- A C compiler and CMake. `crates/bones-engine/bones-kernel` builds SDL3 from source; any compiler works (MSVC, clang, clang-cl, gcc), it is not MSVC-specific.

```sh
pwsh dist.ps1     # engine + reference extension into dist/, plus a release archive
pwsh test.ps1     # fixtures, gates, and every test
```

`pwsh dist.ps1 -NoArchive` stops after `dist/`, for the inner loop of running the engine you just built.

Prefer those scripts over bare `cargo` for anything that touches `crates/bones-engine/bones-kernel`: they set up the compiler and cmake generator through [scripts/native-build-env.ps1](scripts/native-build-env.ps1). A bare `cargo build` on a machine with no generator configured fails inside SDL's cmake with a confusing error.

## Tests

`pwsh test.ps1` is the whole suite and must be green before you open a pull request. There is no CI ([docs/roadmap.md](docs/roadmap.md) tracks adding it), so this script is where every gate is enforced. In order, it:

1. builds the `.wasm` fixtures the integration tests load by path;
2. checks formatting, per package — not `cargo fmt --all`, which would also rewrite the vendored `pubsub-bus` submodule;
3. runs `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
4. runs the tests twice, with default features and with `--all-features`, because only the second compiles and exercises the optional wry web panels;
5. runs the same checks for `bones-messages` and `bones-wasm-sdk`, which the root workspace excludes;
6. builds the documentation with warnings denied.

Some tests open a real SDL window or drive a wry panel. Those run on one dedicated thread that owns SDL for the whole binary, declared with the `sdl_test!` macro rather than `#[test]`. SDL treats whichever thread first calls `SDL_Init` as its main thread and asserts on every later pump from a different one, and libtest gives each test a freshly spawned thread — so a new SDL-touching test that uses a bare `#[test]` will pass alone and abort the suite. Use the macro.

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
