# ADR-002: Engine-owned rendering via draw commands

## Problem

Extensions (game modules, GUI modules) must produce visuals, but WASM
extensions cannot touch SDL or the GPU directly. Where does rendering live?

## Decision

The core owns the SDL renderer. Extensions emit **draw commands** as messages;
the renderer executes them each frame. The command set is a versioned core API.

## Rationale

- Extensions stay pure logic: portable, sandbox-friendly, trivially reloadable.
- One renderer means consistent behavior and a single place to optimize.
- Draw commands travel over the existing message bus — no second channel.

## Rejected alternatives

- **Extensions render into pixel buffers** — maximum freedom, but slow for
  games, and every extension reinvents drawing. May return later as an
  escape-hatch surface type via a new ADR.
- **Extensions call the GPU/SDL directly** — breaks sandboxing and the
  any-language promise.
