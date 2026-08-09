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
