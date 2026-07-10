# ADR-008: Layered input focus — top layer consumes

## Problem

gfx surfaces, egui widgets, and web panels share one window. When the user
types or clicks, exactly one of them should react — typing into a text field
must not also steer the game underneath.

## Decision

Input events are routed through a fixed layer order, top to bottom:

1. **Web panels** (when open and hit by the event)
2. **egui widget layer** (when it claims the pointer/keyboard)
3. **gfx scene** — delivered as `input/*` bus messages to subscribers

Each layer may **consume** an event or pass it through; unconsumed events fall
to the next layer. Only events that reach the bottom appear on `input/*`
topics.

## Rationale

- Matches user expectations: what is visually on top gets the input.
- Matches how egui-style integrations already gate input ("UI wants pointer /
  keyboard"), so the core reuses a proven pattern instead of inventing one.
- Extensions need no focus protocol at all in the common case.

## Rejected alternatives

- **Explicit focus owner via bus messages** — predictable and scriptable, but
  forces every extension to manage focus; disproportionate protocol cost.
- **Broadcast to all subscribers** — pushes the filtering problem onto every
  extension and guarantees input-bleed bugs.
