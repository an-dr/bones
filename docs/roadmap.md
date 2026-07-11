# Roadmap

Remaining implementation increments, in dependency order. Each rung ends with
a demonstrable result, not just progress.

**Maintenance rule: this document lists only work not yet done.** When a rung
is complete, delete it — git history is the record of what shipped. If the
plan changes, rewrite the rungs; do not annotate them.

The order below is a dependency ordering, not a strict sequence: after rung 2,
the renderer, ui, web, and watchdog rungs can proceed in parallel.

| # | Increment | Demo that proves it |
| - | --------- | ------------------- |
| 1 | minimal app: builder skeleton (`extensions_dir`, sleep-loop `run`) | Drop a `.wasm` extension in a directory, run the app, watch it tick/log continuously — no window yet |
| 2 | platform: SDL window, input events onto the bus | Extension logs keypresses |
| 3 | renderer module + `gfx/*` vocabulary | Extension draws a sprite — the windowed app distribution exists from here |
| 4 | Synchronous send (ADR-010), `core/lifecycle`, hot reload | Live level reload: edit a level extension, watch it swap in |
| 5 | Watchdog and budgets (ADR-007) | Runaway extension is faulted and quarantined; engine keeps running |
| 6 | ui module (egui) + `ui/*` vocabulary | The "notes" example ([examples/egui-app.md](examples/egui-app.md)) runs |
| 7 | web module (wry) + `web/*` vocabulary | The "dashboard" example ([examples/web-app.md](examples/web-app.md)) runs |
| 8 | Full builder API: custom native-module injection, embedding demo | A parent project injects a custom module and builds its own engine binary |

Guardrails while climbing:

- **Contract churn is free pre-1.0** — platform, renderer, synchronous-send/
  lifecycle, watchdog, and ui each extend the WIT package; break it rather
  than accumulate compatibility shims (per ADR-011's stability stance).
- **Thin slices within a rung** — Component Model and toolchain setup
  friction (new deps, new host wiring) must not grow a slice; ship the
  smallest demonstrable thing and move on.
- **Every rung ships a runnable demo, not just tests** — the "Demo that
  proves it" column is a deliverable, not a description. Unit tests prove
  correctness; a runnable example (or equivalent observable artifact) proves
  the rung actually does the demonstrable thing, and is what verifies the
  rung is done.
