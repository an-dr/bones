# Roadmap

Remaining implementation increments, in dependency order. Each rung ends with
a demonstrable result, not just progress.

**Maintenance rule: this document lists only work not yet done.** When a rung
is complete, delete it — git history is the record of what shipped. If the
plan changes, rewrite the rungs; do not annotate them.

The order below is a dependency ordering, not a strict sequence: platform,
renderer, watchdog, synchronous send/lifecycle, ui, and the module/service
registry are all done.

| # | Increment | Demo that proves it |
| - | --------- | ------------------- |
| 1 | web module (wry) + `web/*` vocabulary | The "dashboard" example ([examples/web-app.md](examples/web-app.md)) runs |

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
