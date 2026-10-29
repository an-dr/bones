# Agent Context

Notes for AI agents working on this repo that cannot be deduced from the code alone.

## Primary instructions

- Use the globally installed base agent policy, when your tools have one
- Use `AGENTS.md` in the repo root and in the subfolders as scoped extensions of the base rules
- Priority (later entries extend or overwrite earlier ones):
  1. the global base policy, when installed
  2. `REPO/AGENTS.md` — this file
  3. `REPO/**/AGENTS.md` — any subdirectory AGENTS.md, chained by depth

## Working on the engine

Load the `bones-dev` skill ([agent-plugins/bones-dev](agent-plugins/bones-dev/skills/bones-dev/SKILL.md)): where things live, the gates, how a message changes, and the invariants of ADR-044.

## Conventions

- **One type per file. Tests live in their own file.** See [docs/code-style.md](docs/code-style.md) for the precise pattern (the submodule shape, the `command.rs` dispatcher convention, stated exceptions) and worked examples from this codebase. Applies to new code and to code being substantially touched; not a mandate to churn untouched files.
- **Every crate has a README.md** — see docs/code-style.md for what it should cover.
- **Keep bones AI-friendly (ADR-044).** A new core topic comes with its entry in `wit/wire-format.json` (rendered by `cargo xtask wire-format`), and vectors (`cargo xtask vectors`). A new failure an extension can cause comes with a diagnostic that names the cause and the fix. A new capability comes with a way to observe it from outside the process. Everything AI-specific lives in `agent-plugins/` (Agent Plugins 1.0.0): a rule an agent building a product would get wrong goes into `bones-user`'s `references/rules.md`, one an agent working on the engine would get wrong into `bones-dev`.
- **Doc comments: structure over paragraphs.** Prefer short bullet points or a couple of tight sentences to a prose paragraph explaining a decision — split into "what" / "why" bullets rather than one dense block.

## Commit scopes

- web
- wit
- messages
- sdk
- examples
- wasm
- kernel
- devtools
- test
- cli
- docs
- bench
- plugins
- xtask
- release
