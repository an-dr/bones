# Agent Context

Notes for AI agents working on this repo that cannot be deduced from the code alone.

## Primary instructions

- Use `agents/AGENTS.md` as the base instruction
- Use `AGENTS.md` in the repo root and in the subfolders as scoped extensions of the base rules
- Priority (later entries extend or overwrite earlier ones):
  1. `REPO/agents/AGENTS.md` — base
  2. `REPO/AGENTS.md` — this file
  3. `REPO/**/AGENTS.md` — any subdirectory AGENTS.md, chained by depth
