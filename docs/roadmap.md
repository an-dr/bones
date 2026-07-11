# Roadmap

Remaining implementation increments, in dependency order. Each rung ends with
a demonstrable result, not just progress.

**Maintenance rule: this document lists only work not yet done.** When a rung
is complete, delete it — git history is the record of what shipped. If the
plan changes, rewrite the rungs; do not annotate them.

The order below is a dependency ordering, not a strict sequence: after rung 3,
the renderer, ui, web, and watchdog rungs can proceed in parallel.

| # | Increment | Demo that proves it |
| - | --------- | ------------------- |
| 1 | Kernel skeleton, headless: logging, bus, runner loop | Tick loop with native test endpoints; ADR-009 delivery semantics under test |
| 2 | host + minimal contract (`init`, `log`, `on-tick` only) | A WASM hello extension logs through the engine |
| 3 | platform: SDL window, input events onto the bus | Extension logs keypresses |
| 4 | renderer module + `gfx/*` vocabulary | Extension draws a sprite — the app distribution exists from here |
| 5 | Synchronous send (ADR-010), `core/lifecycle`, hot reload | Live level reload: edit a level extension, watch it swap in |
| 6 | Watchdog and budgets (ADR-007) | Runaway extension is faulted and quarantined; engine keeps running |
| 7 | ui module (egui) + `ui/*` vocabulary | The "notes" example ([examples/egui-app.md](examples/egui-app.md)) runs |
| 8 | web module (wry) + `web/*` vocabulary | The "dashboard" example ([examples/web-app.md](examples/web-app.md)) runs |
| 9 | Public builder API + embedding demo | A parent project injects a custom module and builds its own engine binary |

Guardrails while climbing:

- **Contract churn is free pre-1.0** — rungs 2–7 each extend the WIT package;
  break it rather than accumulate compatibility shims (per ADR-011's
  stability stance).
- **Thin slices within a rung** — especially rung 2: one export, one import,
  ship the hello, move on. Component Model setup friction must not grow the
  slice.
