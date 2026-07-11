# ADR-010: Synchronous send — request/reply completes within the call

## Problem

`send(endpoint, payload, deadline) → reply` does not say *when* the target's
`on-message` runs. If requests queue and dispatch on the next cycle, a round
trip costs up to a frame of latency — fine for rare handshakes, unusable for
query-style calls (e.g. a game core asking a level extension for data). The
accidental behavior would become the contract (as ADR-009 argues for
delivery), so dispatch timing must be stated.

## Decision

`send` is **synchronous**: the caller blocks inside the call and the reply is
its return value.

- The host dispatches the request to the target as soon as the target's
  handler serialization (one handler at a time per extension) allows —
  immediately if the target is idle, after its current handler otherwise.
- The target's `on-message` runs under the **target's own** time budget
  (ADR-007). The **caller's** budget clock is paused while it waits; only the
  wall-clock deadline bounds the wait. A slow target faults the target, never
  the caller.
- If the chain of blocked callers would form a cycle (A→B→A), the host fails
  the newest `send` immediately with an error reply — no deadlock, no waiting
  out the deadline.
- Unchanged from ADR-009: every request ends in a reply, an error reply, or a
  deadline error.

## Rationale

- A round trip is bounded by the target's availability, not by the frame
  loop: microseconds in the common case. This makes request/reply usable for
  intra-frame queries, which the chunky-boundary pattern still discourages on
  hot paths but must not forbid.
- Pausing the caller's budget keeps fault attribution honest — ADR-007
  quarantines the extension that misbehaved, and waiting on a peer is not
  misbehavior. The deadline remains the caller's tool for bounding its own
  latency.
- Cycle detection is cheap for the host (it mediates every call and knows who
  blocks on whom) and turns the worst failure mode into an ordinary error
  reply.

## Rejected alternatives

- **Always-queued dispatch, reply as a later `on-message`** — simplest host,
  no blocking, but every round trip costs a dispatch cycle and extension code
  degenerates into correlation-id callback chains; contradicts why ADR-003
  added request/reply.
- **Caller's budget keeps running while blocked** — simpler accounting, but a
  slow target would fault its callers transitively, quarantining the wrong
  extension.
- **Synchronous only when the target is idle, error when busy** — avoids
  blocking machinery, but makes success depend on scheduling luck; callers
  would wrap every send in retry loops, which is the deadline's job.
