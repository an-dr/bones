# Building

## What you need

- A **Rust toolchain**, plus the `wasm32-wasip2` target for extensions: `rustup target add wasm32-wasip2`.
- **PowerShell 7+** (`pwsh`) — cross-platform, and the only script runtime this repository uses.
- **A C compiler and CMake.** `crates/bones-engine/bones-kernel` builds SDL3 from source. Any compiler works — MSVC, clang, clang-cl, gcc; nothing here is MSVC-specific.

Clone with `--recurse-submodules`: `vendor/pubsub-bus` is one.

## The two scripts

```sh
pwsh dist.ps1     # engine + reference extension into dist/, plus a release archive
pwsh test.ps1     # fixtures, gates, and every test
```

`pwsh dist.ps1 -NoArchive` stops after `dist/`, which is the inner loop for running the engine you just built.

## Prefer the scripts over bare cargo

For anything touching `crates/bones-engine/bones-kernel`, use the scripts. They configure the compiler and the cmake generator through [scripts/native-build-env.ps1](../../scripts/native-build-env.ps1). A bare `cargo build` on a machine with no generator configured fails deep inside SDL's cmake with an error that does not point at the cause.

Testing, and the traps specific to it, are in [testing.md](testing.md).
