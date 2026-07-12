# Roadmap

Remaining implementation increments, in dependency order. Each rung ends with
a demonstrable result, not just progress.

**Maintenance rule: this document lists only work not yet done.** When a rung
is complete, delete it — git history is the record of what shipped. If the
plan changes, rewrite the rungs; do not annotate them.

The order below is a dependency ordering, not a strict sequence: ui, web,
and watchdog can all proceed in parallel from here — platform and renderer
(their shared prerequisites) are done.

| # | Increment | Demo that proves it |
| - | --------- | ------------------- |
| 1 | Synchronous send (ADR-010), `core/lifecycle`, hot reload | Live level reload: edit a level extension, watch it swap in |
| 2 | Watchdog and budgets (ADR-007) | Runaway extension is faulted and quarantined; engine keeps running |
| 3 | ui module (egui) + `ui/*` vocabulary | The "notes" example ([examples/egui-app.md](examples/egui-app.md)) runs |
| 4 | web module (wry) + `web/*` vocabulary | The "dashboard" example ([examples/web-app.md](examples/web-app.md)) runs |
| 5 | Full builder API: custom native-module injection, embedding demo | A parent project injects a custom module and builds its own engine binary |

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
