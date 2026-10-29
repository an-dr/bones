# xtask

The repository's build automation, run as `cargo xtask <command>` through the alias in [.cargo/config.toml](../.cargo/config.toml). Written in Rust, so the toolchain that builds the engine is the only thing the build needs, on every platform.

- `check-commits <git-log-args>` fails when a commit in the range credits an AI tool as an author; CI runs it on every push and pull request.
- `doctor` checks every prerequisite and reports what is missing with where to install it.
- `test` builds the extension fixtures and runs every gate and the whole test matrix.
- `dist` builds the engine into `dist/` and a checksummed release archive; `--no-archive` stops after `dist/`.
- `example <name>` builds one example into a runnable `dist/` inside its directory; with no name it lists them.
- `plugin-docs` regenerates the `bones-user` plugin's documentation reference from the product-facing documents.
- `plugins` checks every package in `agent-plugins/` against Agent Plugins 1.0.0 and its skills' frontmatter.
- `wire-format` renders `wit/wire-format.json` as `wit/wire-format.md` and writes its JSON Schema, after checking the specification against itself and against the vectors (ADR-041).
- `vectors` regenerates `wit/vectors/vectors.txt` from the `bones-messages` codec. Every changed line is an ABI change, so read the diff.

Each command checks its prerequisites before doing any work and stops at the first missing one. Nothing is installed automatically.

- `prereq.rs` — the checks and their install hints.
- `native.rs` — the compiler and cmake generator setup SDL3's source build needs.
- `process.rs` — running cargo and probing `PATH`.
- `commits.rs`, `dist.rs`, `example.rs`, `test.rs`, `ts_codec.rs`, `vectors.rs`, `wire_format.rs` — one file per command; `example.rs` holds the example table.
- `metadata.rs` — versions and facts read from cargo and `wit/`, never repeated.
- `archive.rs`, `files.rs` — zip, checksum, and copy steps the packaging commands share.
- `templates/` — the sample `bones.toml` and release README that `dist` ships.

Not part of either version line: it ships in no release and nothing depends on it. [docs/contributing/building.md](../docs/contributing/building.md) is the user-facing guide.
