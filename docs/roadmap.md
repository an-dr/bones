# Roadmap

Add future work here only when it has a demonstrable completion artifact;
completed work belongs in git history.

## Desktop OS capabilities as a module

`platform` is documented as the only component touching the OS, but it covers
the SDL window and input devices only. An embedder building a desktop tool
brings its own clipboard access, external-URL opening, and file/folder pickers;
the one bones app that does (a Git client) carries a bus-mediated module for
exactly that, correlating requests and replies the way `files` does.

The shape is already settled by what exists here — a trusted native module
answering direct sends, with a backend trait so tests use a stub instead of a
real desktop. What is not settled is the dependency question ADR-019 framed for
`game-core`: clipboard and native dialogs mean bought dependencies (`arboard`,
`rfd`) that only a desktop composition needs, so this wants an ADR and a
feature-gate decision before code.

Completion artifact: a feature-gated module whose capabilities are exercised
through a stub backend in tests, and a desktop composition that no longer needs
its own copy.

## `bones-` crate prefixes and a facade crate

Every workspace crate carries an unprefixed generic name — `bus`, `runner`,
`platform`, `logging`, `contract`. Fine while they were private workspace
members, but `structure.md` commits to a **library distribution** consumed as a
git dependency, and at that point an embedder's `Cargo.toml` collides with any
other dependency named `logging` or `platform`. The three known embedders
(`commits`, `artificial-will-game-v2`, `copper`) already depend on these crates
by path, so the rename is a breaking change for them.

Two decisions, neither settled. First, whether to rename every crate with a flat
`bones-` prefix. Second, whether to add a thin `bones` facade crate re-exporting
`Engine`/`BuiltEngine`/`Supervisor`, which is what would make structure.md's
"app has no access embedders lack" rule enforced by the dependency graph rather
than by convention — today nothing stops `app` reaching past the intended public
surface into `runner`'s internals.

Deferred rather than dropped: this was drafted once and abandoned, and the draft
is in this repository's history. Renaming is mechanical today and gets more
expensive with every crate added.

Completion artifact: an ADR settling both questions, and — if it decides to
rename — every crate renamed with the embedders' path dependencies updated in
the same pass.
