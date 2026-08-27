# Testing

`pwsh test.ps1` is the whole suite, and it must be green before a pull request. There is no CI ([roadmap.md](../roadmap.md) tracks adding it), so this script is where every gate is actually enforced — nothing else checks these.

## What the script runs

In order:

1. Builds the `.wasm` fixtures the integration tests load by path.
2. Checks formatting **per package** — not `cargo fmt --all`, which would also rewrite the vendored `pubsub-bus` submodule.
3. Runs `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
4. Runs the tests **twice**, with default features and with `--all-features`. Only the second compiles and exercises the optional wry web panels.
5. Runs the same checks for `bones-messages` and `bones-wasm-sdk`, which the root workspace excludes so they can build for `wasm32-wasip2`.
6. Builds the documentation with warnings denied.

## The SDL main-thread trap

Some tests open a real SDL window or drive a wry panel. **Declare those with the `sdl_test!` macro, never a bare `#[test]`.**

SDL treats whichever thread first calls `SDL_Init` as its main thread and asserts on every later pump from a different one. libtest gives each test a freshly spawned thread. So a new SDL-touching test written with `#[test]` **passes when run alone and aborts the whole suite when run with the others** — a failure that looks like flakiness and is not. The macro routes every such test onto one dedicated thread that owns SDL for the binary's lifetime.
