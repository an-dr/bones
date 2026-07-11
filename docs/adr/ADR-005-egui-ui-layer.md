# ADR-005: egui UI layer in the core, widgets as bus messages

## Problem

SDL provides no widgets. GUI-module extensions need an established UI toolkit,
but extensions live behind the WASM boundary and can only speak messages —
they cannot call a toolkit directly.

## Decision

The core embeds **egui** as a UI layer next to the renderer. Extensions
declare widgets by publishing a widget spec on `ui/*` topics each frame
(immediate mode maps directly onto `on-tick` → publish spec → core renders);
the core publishes interaction events back (`ui/clicked`, `ui/changed`).
egui's output (textured triangles) is drawn by the existing SDL renderer —
no second rendering path.

The widget vocabulary is a versioned public API, deliberately kept small at
first (on the order of a dozen core widgets).

## Rationale

- Pure Rust, mature, backend-agnostic — fits the core with no C++ dependency.
- Immediate mode fits the event-driven execution model (ADR-004) naturally.
- This is ADR-002 ("engine presents, extensions send commands") applied one
  abstraction level up; GUI extensions stay portable and language-agnostic.

## Rejected alternatives

- **Dear ImGui (imgui-rs)** — equally proven, but adds a C++ dependency to an
  otherwise pure-Rust core for no capability gain over egui.
- **Extensions build UI from raw `gfx/*` commands** — every extension
  reinvents widgets, layout, and input handling.
- **Retained-mode widget tree with diffs** — less per-frame traffic but a much
  fatter protocol; may supersede this via a new ADR if the per-frame spec ever
  becomes a measured bottleneck.
