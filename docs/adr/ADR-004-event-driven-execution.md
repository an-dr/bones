# ADR-004: Event-driven extension execution with optional tick

## Problem

Something must decide when extension code runs. Game modules want a frame loop; GUI modules should be idle-cheap. One execution model has to serve both README use cases.

## Decision

Extensions are event-driven: they export handlers (`init`, `on-message`, `on-tick`, `shutdown`) that the core calls. Extensions never own a thread or a loop. Those needing a frame loop subscribe to the `core/tick` topic and receive `on-tick(dt)` callbacks.

## Rationale

- GUI-style extensions consume nothing while idle.
- Game-style extensions get a deterministic per-frame callback with delta time.
- A single dispatch model keeps the host simple and scheduling in one place.

## Rejected alternatives

- **Pure game loop (`update(dt)` for everyone)** — GUI extensions burn CPU idling and messaging degrades into polling.
- **Extensions own their loop on dedicated threads** — closest to real actors, but threading plus a wasm store per thread adds runtime complexity that nothing currently requires.
