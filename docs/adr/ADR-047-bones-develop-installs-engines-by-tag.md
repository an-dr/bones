# ADR-047: bones-develop installs engines by tag

## Problem

Releases carry no binaries (ADR-046), so a user who wants the engine clones a tag and runs `cargo xtask dist`, and a product project has no link to the engine it was written for beyond the tags in its manifests. ADR-043 shipped the developer tools beside the engine in every archive and kept them on the engine version line; with no archive, nothing puts the right engine beside the tools.

## Decision

The developer tools are `bones-develop`, installed once with `cargo install --git https://github.com/an-dr/bones bones-develop`. It builds and caches engines, and it is the way to start or extend a product.

- **The cache.** Everything lives under `BONES_HOME`, `~/.bones` by default. `sources/<tag>` is a shallow clone of the tag with its submodules, `engines/<tag>` or `engines/<tag>-web` holds the built `bones` executable with that tag's `wit/` beside it, and `target/` is one cargo target directory shared by every build, so a later tag rebuilds only what changed.
- **Engines come from source.** `engine install <tag> [--web]` clones the tag with `git` and runs `cargo build --release -p bones`. `BONES_REPOSITORY` replaces the GitHub URL, for a mirror, a fork, or a local repository in tests and CI. `engine list` shows the cache and the repository's `v*` tags; `engine remove <tag>` deletes a tag's engines and source.
- **Two ways to install bones in a project.** By default a project pins tags and its engine is built in the shared cache: one engine per tag per machine. With `--submodule`, `new` and `add` vendor bones as a git submodule checked out at the tag, the project's dependencies are paths into it, and its engine is built from it into the project's own `.bones/engine`; a clone with `--recursive` then holds everything the project needs. `upgrade <tag>` moves either kind to another tag.
- **The project picks its engine.** A command that runs an engine takes `--engine`, then `BONES_ENGINE`, then the engine of the bones checkout the project's path dependencies point into, then the engine for the `v*` tag the project pins, building it on first use. The web variant is used when the project's `bones.toml` enables web panels.
- **A tag's files come from that tag.** The agent rules and MCP configuration a project receives, and the wire format the MCP server translates with, are read from the tag's source; the copies compiled into `bones-develop` are only the fallback for a project with a path dependency.
- **The tool has its own life.** One `bones-develop` drives engines of several versions, and `self-update` reinstalls it with `cargo install --force`. It keeps the workspace version, but a project's pins come from the tag it chose, never from the tool's own version.

This supersedes the rest of ADR-043: the tools are no longer shipped beside the engine and no longer pin the release they came from. Their separation from the `bones` binary and their shared `bones-build` crate stand.

## Rationale

Building at a tag is what a release now is, so the tool that runs the engine is the one place to do it, and caching by tag lets every project on a machine keep its own engine without a clone of this repository. Reading files from the tag rather than from the tool keeps an older project's agent guidance and message contract consistent with its engine, whichever `bones-develop` is installed.

## Rejected alternatives

- Downloading prebuilt engines: releases carry no binaries (ADR-046).
- One global engine on PATH: two projects pinned to different tags could not both run.
- Cloning with a Rust git library: `git` is already required by cargo's git dependencies, and a library would add a native dependency for one command.
- A cargo target directory per tag: every tag would rebuild the whole dependency tree from scratch.
- Only one way to install: the cache suits a developer with several projects, while a submodule suits a project that must build from its own clone, offline or in CI, so both are offered.
