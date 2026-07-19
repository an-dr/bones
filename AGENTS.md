# Agent Context

Notes for AI agents working on this repo that cannot be deduced from the code alone.

## Primary instructions

- Use `agents/AGENTS.md` as the base instruction
- Use `AGENTS.md` in the repo root and in the subfolders as scoped extensions of the base rules
- Priority (later entries extend or overwrite earlier ones):
  1. `REPO/agents/AGENTS.md` — base
  2. `REPO/AGENTS.md` — this file
  3. `REPO/**/AGENTS.md` — any subdirectory AGENTS.md, chained by depth

## Conventions

- `docs/reviews/` (the `code-review` skill's output) is local-only —
  gitignored, never committed. It's working material for the current
  session, not project history.
- **One type per file. Tests live in their own file.** See
  [docs/code-style.md](docs/code-style.md) for the precise pattern (the
  submodule shape, the `command.rs` dispatcher convention, stated
  exceptions) and worked examples from this codebase. Applies to new code
  and to code being substantially touched; not a mandate to churn
  untouched files — see [docs/roadmap.md](docs/roadmap.md) for the
  tracked repo-wide sweep.
- **Every crate has a README.md** — see docs/code-style.md for what it
  should cover.
- **Doc comments: structure over paragraphs.** Prefer short bullet points
  or a couple of tight sentences to a prose paragraph explaining a
  decision — split into "what" / "why" bullets rather than one dense block.
