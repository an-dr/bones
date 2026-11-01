# bones-develop

The tool a user installs to get bones and to work on a product built on it (ADR-047). This page is for working on the tool itself. Using it is in the [manual](../../docs/manual/bones-develop.md), and what it must keep doing is in the [use cases](../../docs/use-cases/bones-develop.md).

## Boundaries

- **It never links the kernel** (ADR-043). It depends on `bones-messages` for the ABI surface, `bones-build` for building extensions, and `serde_json`; it runs the engine as a separate process and talks to it over the devtools channel.
- **A project's files come from the release it pins** (ADR-047). Templates, agent rules, the MCP configuration, the ABI version, and the wire format are read from that release's source; the copies compiled in from this repository are the fallback, for a project on a local checkout.
- **Engines are built, never downloaded** (ADR-046). `git` and `cargo` are the only external programs it runs.

## Modules

| Module | Owns |
| --- | --- |
| `main` | Dispatch and the usage text; every command reports failure as one message saying what to do |
| `new_project`, `kind`, `dependencies` | Writing a project: its files, kinds, and pins |
| `source` | Reading files from a bones checkout, with the compiled-in fallback; templates under the current and the former crate name |
| `engines`, `home`, `repository` | The cache under `BONES_HOME`: shallow clones by tag, one shared target, built engines with their `wit/` |
| `engine`, `project` | Which engine a project runs: `--engine`, `BONES_ENGINE`, a checkout its path dependencies point into, then its pinned tag |
| `submodule` | Vendoring bones as a git submodule at a tag, and moving it |
| `add`, `upgrade`, `self_update` | The commands of the same names |
| `dev`, `screenshot`, `doctor` | The development loop, captures, and diagnosis |
| `mcp`, `devtools_client`, `catalogue`, `base64`, `hex` | The MCP server, the devtools channel client, the wire-format encoder read from the engine's `wit/`, and the encodings its tools use |

## Tests

```sh
cargo test -p bones-develop
```

Unit tests sit beside each module in its own `tests.rs`. They need `git` (one clones a repository it creates in the temporary folder) but no network and no engine build. The engine build and the pinned-engine path are checked end to end in CI, where every platform runs `bones-develop engine install` of the pushed commit; locally, point `BONES_REPOSITORY` at your checkout, tag a commit, and run the commands against it with `BONES_HOME` outside `%TEMP%` on Windows.

## Changing it

- A new or changed command changes the [manual](../../docs/manual/bones-develop.md) and its [use case](../../docs/use-cases/bones-develop.md) in the same delivery. The manual is copied into the `bones-user` agent plugin by `cargo xtask plugin-docs`.
- Templates in `templates/` are read from a project's release, so a template change reaches only projects on releases that contain it.
