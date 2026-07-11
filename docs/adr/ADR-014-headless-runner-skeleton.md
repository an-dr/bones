# ADR-014: Headless runner skeleton — step-driven, injected bus, virtual clock

## Problem

Rung 1 (roadmap.md) needs a runner that drives `on-tick` and dispatches bus
messages with nothing registered yet — no platform, no renderer, no
extensions. The shape chosen here is what rungs 3/4/9 attach to, so it needs
to be right without over-building ahead of need.

## Decision

- **Run mode**: the runner's primary API is bounded and step-driven —
  `step()` / `run_for(n_ticks)`. "Run forever" (the shipped app's actual
  loop) is a thin wrapper calling `step()` in a loop; it is not the
  primitive tests use.
- **Bus ownership**: the caller constructs the bus and injects it into the
  runner. The runner does not own bus construction.
- **Clock**: `on-tick(dt)` receives a caller-supplied or fake `dt` — no
  wall-clock sleep in the headless runner. Real-time pacing arrives when
  platform (rung 3) drives the loop.
- **Test endpoints**: rung 1's "native test endpoints" (roadmap.md) are a
  throwaway mock-subscriber harness local to tests, not an early
  implementation of the ADR-011 module trait. The module contract is
  designed once, deliberately, at rung 9 with renderer/ui/web experience
  behind it.
- **Phase skeleton**: the runner implements the full five named phases from
  design/modules.md (`input → dispatch → tick → render → present`) now, as
  an enum with no-op hooks where nothing is registered yet. This is a
  low-stakes call (already-documented shape, ~nothing to implement per
  empty phase) made without a separate question; flag if you'd rather grow
  phases incrementally per rung.

## Rationale

- Step-driven + injected bus + virtual clock together make every rung-1
  test deterministic and fast, with no timing flakiness and no coupling
  between bus-semantics tests and runner/loop machinery.
- Deferring the module trait to rung 9 avoids locking in an API before any
  real module (renderer, ui) has exercised it, and keeps rung 1 inside the
  300-line increment budget.
- Implementing all five phases now means later rungs slot in without
  reshaping the loop — the phase list becomes stable from day one.

## Rejected alternatives

- **Always-run loop, stopped only by an explicit shutdown message** — no
  shutdown source exists until rung 3/5; every test would need to manufacture
  one just to return.
- **Runner owns and constructs the bus** — couples bus-semantics tests to
  the runner's existence, contradicting structure.md's component separation.
- **Real wall-clock timing from day one** — makes rung-1 tests timing-
  sensitive for a capability (real-time pacing) nothing needs until rendering
  exists.
- **Reuse the ADR-011 module trait for test endpoints now** — would smuggle
  module-system design into a kernel-skeleton rung, ahead of the experience
  rung 9 is meant to draw on.
