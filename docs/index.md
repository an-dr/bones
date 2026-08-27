# Docs index

Map of the documentation in this repository.

## Two rules

**The documentation is the authority.** It records intent; code only exhibits behaviour, and reading intent is cheaper than reconstructing it. Where code and documentation disagree, the code is wrong. A change in behaviour updates the document first and the code after — revising a decision is expected, a document lagging behind the code is not.

**Each layer has an altitude, and the layers do not overlap.**

- [architecture/](architecture/index.md) — concepts: what the parts are, where the boundaries fall, what the system guarantees.
- `design/` — behaviour contracts for one subsystem: semantics, guarantees, state machines. Never exact signatures or file layout.
- `adr/` — decisions with rationale. Immutable; superseded by later ADRs, never edited.
- `contributing/` — how to build, test, and release the repository.
- [roadmap.md](roadmap.md) — remaining work only; completed work is deleted rather than checked off.

The altitude test: **an ordinary refactoring must not require a documentation update.** Documentation changes when observable behaviour, a contract, or a component boundary changes — not when code moves, splits, or is renamed. A page that needs editing during refactorings is written too low; raise it rather than maintain it.

**Stated exception**: [code-style.md](code-style.md) documents file-layout conventions themselves. It is code-level by definition and changes only when the conventions change, never as a side effect of applying them.

**Tutorials are not here.** Getting started, writing an extension, and embedding are code-level by nature — exact commands, exact paths — so they live in READMEs beside the code they describe, which is what keeps the altitude rule above intact rather than carving another exception into it.

## Start here

- [README](../README.md) — what bones is, prerequisites, quickstart.
- [architecture/](architecture/index.md) — the system: parts, structure, messaging, upgrading.
- [glossary.md](glossary.md) — host, guest, embedder, native module, service, and where new code goes.
- [CONTRIBUTING](../CONTRIBUTING.md) — how to build, test, and land a change.

## Architecture

The chapter to read to understand the current state of the system, or to propose a change to it.

- [architecture/index.md](architecture/index.md) — the chapter map and the rules above.
- [architecture/overview.md](architecture/overview.md) — kernel, native modules, extensions; the shipped modules, the three presentation levels, the two public surfaces.
- [architecture/structure.md](architecture/structure.md) — component inventory, dependency rules, source layout, distributions.
- [architecture/messaging.md](architecture/messaging.md) — bus topology, guarantees, the frame loop, shutdown.
- [architecture/upgrading.md](architecture/upgrading.md) — how a shipped application replaces itself.

Supporting:

- [glossary.md](glossary.md) — the vocabulary the other documents assume.
- [code-style.md](code-style.md) — file-layout conventions for Rust source.

## Detailed design

One subsystem each, one level below architecture.

- [design/messaging.md](design/messaging.md) — envelope, topic namespace, request/reply, delivery, flow control.
- [design/extensions.md](design/extensions.md) — contract, execution, lifecycle, faults, hot reload.
- [design/modules.md](design/modules.md) — module contract, frame phases, services, composition root, embedding.
- [design/presentation.md](design/presentation.md) — gfx/ui/web backends and input routing.
- [design/platform.md](design/platform.md) — window, tray, input, frame loop, shutdown.
- [design/game-core.md](design/game-core.md) — entity operations, multi-world physics, what the simulation publishes.
- [design/audio.md](design/audio.md) — effects and music, the volume convention, silent failure.
- [design/os.md](design/os.md) — clipboard, browser, dialogs and fetch for sandboxed guests.

## Decisions

[adr/index.md](adr/index.md) lists every ADR with its status, including the two that were reversed. Read one when you need to know why something is the way it is, or when you are about to overturn it.

## Examples

Behaviour walkthroughs; runnable code is under [examples/](../examples/README.md).

- [examples/egui-app.md](examples/egui-app.md) — "notes", a widget-UI extension.
- [examples/web-app.md](examples/web-app.md) — "dashboard", a web-panel extension.

Directory-level READMEs explain what belongs where: [crates/](../crates/README.md), [examples/](../examples/README.md), [wit/](../wit/README.md).

## History

The granular pre-release history — 176 commits from 2026-07-10 to 2026-08-09 — is archived at the tag `compat-main-2026-08-09`. The phase commits on `main` are squashed from it.
