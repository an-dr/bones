# xtask

The repository's build automation, run as `cargo xtask <command>` through the alias in [.cargo/config.toml](../.cargo/config.toml). It replaces the PowerShell scripts the repository used to carry, so the Rust toolchain that builds the engine is the only thing the build needs.

- `doctor` checks every prerequisite and reports what is missing with where to install it.
- `test` builds the extension fixtures and runs every gate and the whole test matrix.
- `dist` builds the engine into `dist/` and a checksummed release archive; `--no-archive` stops after `dist/`.
- `example <name>` builds one example into a runnable `dist/` inside its directory; with no name it lists them.

Each command checks its prerequisites before doing any work and stops at the first missing one. Nothing is installed automatically.

- `prereq.rs` — the checks and their install hints.
- `native.rs` — the compiler and cmake generator setup SDL3's source build needs.
- `process.rs` — running cargo and probing `PATH`.
- `dist.rs`, `example.rs`, `test.rs` — one file per command; `example.rs` holds the example table.
- `metadata.rs` — versions and facts read from cargo and `wit/`, never repeated.
- `archive.rs`, `files.rs` — zip, checksum, and copy steps the packaging commands share.
- `templates/` — the sample `bones.toml` and release README that `dist` ships.

Not part of either version line: it ships in no release and nothing depends on it. [docs/contributing/building.md](../docs/contributing/building.md) is the user-facing guide.
