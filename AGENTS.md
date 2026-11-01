# Agent context

Follow the globally installed base agent policy when available. For work on this engine, load the [bones-dev skill](agent-plugins/bones-dev/skills/bones-dev/SKILL.md). For a product built on bones, use the [bones-user skill](agent-plugins/bones-user/skills/bones/SKILL.md).

AI instructions are product behaviour, not documentation: the agent plugins (`agent-plugins/`), the MCP server, `AGENTS.md`, and `CLAUDE.md` change in `feat(ai):` or `fix(ai):` commits, never `docs:`, and their changelog entries go under Added, Changed, or Fixed.

## Commit scopes

- web
- ai
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
