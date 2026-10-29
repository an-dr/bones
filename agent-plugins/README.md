# agent-plugins

Everything bones offers an AI agent, as [Agent Plugins](https://agent-plugins.org/) 1.0.0 packages: one directory per plugin, a `plugin.json`, skills, and an optional `mcp.json`. Any client that implements the standard loads them. Nothing AI-specific lives anywhere else in the repository; other files that agents read are generated from these.

| Plugin | For | Holds |
| --- | --- | --- |
| [bones-user](bones-user/plugin.json) | an agent building a product on bones | the `bones` skill, the product rules as its reference, and the `bones` MCP server (`bones-cli mcp`) |
| [bones-dev](bones-dev/plugin.json) | an agent working on the engine itself | the `bones-dev` skill: the wire format, the gates, and the invariants a change is judged against |

## Use

- **A client that loads Agent Plugins** — install the plugin directory the way the client does; every release archive carries `agent-plugins/bones-user/`.
- **A client that only reads skills** — copy a plugin's `skills/<name>/` folder into the client's skills directory, such as `~/.claude/skills/` for Claude Code.
- **A client that reads `AGENTS.md` and `.mcp.json`** — `bones-cli new` writes both into every project, generated from `bones-user`: `AGENTS.md` is its rules reference, and `.mcp.json` its MCP server with the path of the `bones-cli` that wrote it.

The packages hold only the portable core: skills and MCP servers. A client-specific extra would go in a reverse-domain folder inside its plugin, and there are none yet.

## Changing them

- `cargo xtask plugins` checks every `plugin.json` and `mcp.json` against the 1.0.0 rules, and every skill's frontmatter; the workspace tests run the same check.
- A plugin's `version` follows the engine line, since `bones-user` describes the engine and the tools of that release.
- A rule an agent building a product would get wrong goes into `bones-user/skills/bones/references/rules.md` (ADR-044).
