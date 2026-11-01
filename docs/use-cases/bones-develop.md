# Use cases: bones-develop

`bones-develop` is the tool a user installs to get bones and to work on a product built on it ([ADR-047](../adr/ADR-047-bones-develop-installs-engines-by-tag.md)). How to use each command is in the [manual](../manual/bones-develop.md).

A project installs bones in one of two ways. **Local machine**, the default: it pins `v*` and `abi-v*` tags, and its engine is built once per tag in `~/.bones`. **Submodule**: it vendors bones as a git submodule at a tag, depends on paths into it, and builds its engine from it.

Two kinds of product appear below. An **extension product** is one or more WASM extensions that the unchanged `bones` engine loads. A **native application** is the developer's own Rust program that links the engine as a library and composes it in code.

## Getting bones

### UC-DEV-01: Install the tools

- **Actor**: product developer, AI agent.
- **Goal**: get bones onto a machine.
- **Flow**: `cargo install --git https://github.com/an-dr/bones bones-develop`.
- **Success**: `bones-develop --help` lists the commands; nothing else was installed and no clone of the repository is needed.

### UC-DEV-12: Manage the engines built on this machine

- **Actor**: product developer.
- **Goal**: an engine is built from source the first time a project needs it, which takes minutes. The developer wants to build it beforehand (before going offline, or before a demo), see which engines the machine holds, and delete the ones no project uses any more.
- **Flow**: `bones-develop engine install <tag> [--web]` builds one; `engine list` shows the built engines and the release tags available; `engine remove <tag>` deletes one.
- **Success**: a project pinned to an installed tag runs without building; the list matches what is on disk; a removed tag no longer takes space.

### UC-DEV-16: Work from a mirror, a fork, or a local repository

- **Actor**: product developer, CI.
- **Goal**: take bones from somewhere other than GitHub.
- **Flow**: set `BONES_REPOSITORY` to that repository, then use any command.
- **Success**: engines, new projects, and agent files all come from that repository.

### UC-DEV-18: Keep the tool current

- **Actor**: product developer.
- **Goal**: run the newest `bones-develop`, or a chosen release of it.
- **Flow**: `bones-develop self-update [--tag <tag>]`.
- **Success**: the reinstalled tool runs; a failed reinstall leaves the old one working.

## Creating a product

### UC-DEV-02: Create an extension product

- **Actor**: product developer.
- **Goal**: a new game, tool, or GUI written as extensions for the released engine.
- **Flow**: `bones-develop new <name> [--kind game|egui|web]`.
- **Success**: the project pins the newest release, its headless checks pass with `cargo test`, and `bones-develop dev` runs the product in the engine.

### UC-DEV-03: Create a product that vendors bones as a submodule

- **Actor**: product developer.
- **Goal**: a new product that builds entirely from its own clone, for a team, CI, or offline work.
- **Flow**: `bones-develop new <name> --submodule [path]`.
- **Success**: bones sits in the project as a submodule at the release tag, and the project builds and runs without `~/.bones`.

### UC-DEV-04: Create a native application that embeds the engine

- **Actor**: application developer.
- **Goal**: a new Rust program of their own that runs the engine as a library, composing its window and modules in code, and can still load extensions.
- **Flow**: `bones-develop new <name> --kind embed`.
- **Success**: the project has an application crate beside its extension and checks, and `bones-develop dev` runs that application.

## Embedding the engine in an existing program

### UC-DEV-05: Use the engine as a library in an existing application

- **Actor**: application developer.
- **Goal**: an existing Rust program gains the bones engine as a library dependency, to compose and run it from its own `main`.
- **Flow**: in the program's crate, `bones-develop add [--tag <tag>] [--submodule [path]]`.
- **Success**: the crate depends on `bones-kernel` at the tag, or on the submodule, and resolves.

## Developing a product

### UC-DEV-07: Develop extensions with rebuild and hot reload

- **Actor**: product developer.
- **Goal**: see each change to an extension running as soon as it is saved.
- **Flow**: `bones-develop dev` in the project, then edit the extension.
- **Success**: the engine the project pins starts (built on first use) with the extension loaded, and every save rebuilds the extension and reloads it in the running engine.

### UC-DEV-08: Take a screenshot of the running product

- **Actor**: product developer, CI.
- **Goal**: an image of what the product shows on screen, for documentation, a design check, or a test, without a display.
- **Flow**: `bones-develop screenshot [--press <key>]... [--frames <n>] [--window]`.
- **Success**: a PNG of the product's window contents after the given key presses and number of rendered frames; nothing appears on screen unless `--window` is given.

### UC-DEV-09: Let an AI agent run, observe, and drive the product

- **Actor**: AI agent.
- **Goal**: build, run, inspect, and steer the product through tools rather than by reading engine source.
- **Flow**: the agent's client runs `bones-develop mcp` from the project's `.mcp.json`, then calls its tools.
- **Success**: the agent launches the engine, publishes messages as JSON fields, reads bus traffic, logs, draw commands, and screenshots, and steps time, in the message format of the engine the project pins.

### UC-DEV-10: Find out what is wrong with the setup

- **Actor**: product developer, AI agent.
- **Goal**: know why the project does not build or run, and what to do.
- **Flow**: `bones-develop doctor`.
- **Success**: each missing prerequisite or mismatch is listed with its fix; the engine the project uses, and whether it is built, is named.

### UC-DEV-11: Keep several projects on different engine versions on one machine

- **Actor**: product developer.
- **Goal**: work on an old project and a new one without them disturbing each other.
- **Flow**: use each project's commands as usual.
- **Success**: each project runs the engine of the tag it pins, side by side.

## Keeping a project current

### UC-DEV-13: Move a project to a newer release

- **Actor**: product developer.
- **Goal**: take a new bones release into an existing project.
- **Flow**: `bones-develop upgrade <tag>`.
- **Success**: the project pins the new tag and its ABI line, or its submodule is at the new tag; its `AGENTS.md` and `.mcp.json` come from the new tag, so the agent guidance matches the engine; the next `dev` runs the new engine.

### UC-DEV-15: Try unreleased bones

- **Actor**: bones maintainer, early adopter.
- **Goal**: use work that is not released yet.
- **Flow**: `bones-develop new <name> --bones-path <checkout>`, or `engine install main`.
- **Success**: the project builds against the checkout or `main`, and its engine is built from that source.

## Maintaining bones

### UC-DEV-19: Check that the engine builds on every platform

- **Actor**: bones maintainer, CI.
- **Goal**: know that users can build the engine everywhere bones claims to run.
- **Flow**: CI runs `bones-develop engine install` of the pushed commit on Windows x64 and ARM64 and Linux x64 and ARM64, with and without web panels.
- **Success**: every variant builds; a failure names its platform before a release is tagged.
