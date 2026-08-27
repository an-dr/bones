# Contributing

Thanks for looking. Open an issue before starting anything large — [docs/roadmap.md](docs/roadmap.md) is short on purpose, and some things are deliberately deferred rather than missing.

## Getting set up

- [docs/contributing/building.md](docs/contributing/building.md) — toolchain, prerequisites, and the two scripts.
- [docs/contributing/testing.md](docs/contributing/testing.md) — what `test.ps1` gates, and the SDL main-thread trap that will bite you if you skip it.
- [docs/contributing/releasing.md](docs/contributing/releasing.md) — cutting a release on either version line.

`pwsh test.ps1` must be green before you open a pull request. There is no CI, so it is the only thing that checks.

## Understanding the project

- [docs/architecture/](docs/architecture/index.md) — what the parts are and why the boundaries fall where they do. Start here.
- [docs/design/](docs/design/) — the behaviour of one subsystem in detail.
- [docs/adr/index.md](docs/adr/index.md) — the decisions, with their status.

## The rules that govern a change

Each of these is stated in one place, and that place is authoritative — what follows is only a pointer to it.

- **Code conventions** — [docs/code-style.md](docs/code-style.md). One type per file, tests out of line, a README per crate, doc comments as bullets rather than prose. They apply to new code and to code you substantially touch, and are not a mandate to churn files you are only passing through.
- **Documentation** — [docs/architecture/index.md](docs/architecture/index.md) states the two rules: documentation is the authority, and architecture holds concepts while design holds implementation. [docs/index.md](docs/index.md) maps what lives where.
- **Decisions** — [docs/adr/index.md](docs/adr/index.md). Immutable once recorded, superseded rather than edited, and only for lasting architectural choices.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/): a `type: imperative summary` subject of 72 characters or less, lowercase, no trailing period, optionally followed by a blank line and `- ` body entries explaining cause or motivation rather than restating the diff.

```text
fix: raise the minimum window width to 1200
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `style`, `chore`, `build`, `ci`, `perf`, `revert`. Needing two types means the commit does two things and should be split. Mark a breaking change with `!` and a `BREAKING CHANGE:` footer.

Commit messages record who is accountable for a change, so do not credit tooling in them — no assistant or generator trailers.
