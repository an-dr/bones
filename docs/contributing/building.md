# Building

## What you need

- A **Rust toolchain**, plus the `wasm32-wasip2` target for extensions: `rustup target add wasm32-wasip2`.
- **A C compiler and CMake.** `crates/bones-kernel/bones-kernel-infra` builds SDL3 from source. Any compiler works — MSVC, clang, clang-cl, gcc; nothing here is MSVC-specific. Ninja is used when present.

- **On Linux, the development packages SDL3, audio, and the web panels link against** — X11, Wayland, ALSA, PulseAudio, GTK 3, and webkit2gtk among them. On Debian and Ubuntu, `cargo xtask doctor` checks each one, and `sudo apt-get install $(cargo xtask linux-packages)` installs them all; on other distributions, install the equivalents of the names that command prints.

Clone with `--recurse-submodules`: `vendor/pubsub-bus` is one.

Nothing else. The automation is a Rust program in [xtask/](../../xtask/), run through a cargo alias, so the toolchain that builds the engine also runs its build scripts.

## The commands

```sh
cargo xtask doctor          # check every prerequisite and say how to install what is missing
cargo xtask dist            # engine and ABI into dist/, plus a release archive
cargo xtask test            # fixtures, gates, and every test
cargo xtask example <name>  # build one example into its own dist/ and say how to run it
cargo xtask check-commits <range>  # reject commits that credit an AI tool as an author
cargo xtask linux-packages     # the Debian/Ubuntu packages a Linux build needs
```

`cargo xtask dist --features web` builds the variant with wry web panels and names its archive `-web`. `cargo xtask dist --no-archive` stops after `dist/`, which is the inner loop for running the engine you just built. `cargo xtask example` with no name lists the examples.

## Checking prerequisites

Every command checks what it needs before it starts and stops at the first missing tool with what is missing, why it is needed, and where to get it. It never installs anything: a toolchain is the developer's choice, and a build script that silently provisions one hides what the build depends on. `cargo xtask doctor` runs every check at once and reports all of them.

## Prefer xtask over bare cargo

For anything touching `crates/bones-kernel/bones-kernel-infra`, use xtask. It configures the compiler and the cmake generator for the cargo it runs — preferring a compiler already on `PATH`, and on Windows falling back to loading MSVC through `vcvarsall.bat`. A bare `cargo build` on a machine with no generator configured fails deep inside SDL's cmake with an error that does not point at the cause.

Testing, and the traps specific to it, are in [testing.md](testing.md).