# ADR-015: Deferred dispatch remains mandatory regardless of the pubsub-bus fix

## Problem

ADR-013 states that its two bones-side workarounds (persistent adapter, deferred dispatch) "remain correct once the [pubsub-bus] fixes land (they simply become unnecessary, not wrong)." Implementing the pubsub-bus fix (snapshot subscribers, drop the `RwLock` guard before invoking callbacks) showed this is only true for the adapter/removal workaround. It is false for deferred dispatch, and left uncorrected the claim would mislead a future reader into removing bones' dispatch queue once pubsub-bus is patched, reintroducing a live deadlock.

pubsub-bus locks each subscriber's own `Mutex` not only for `on_event` but also just to call `is_subscribed_to` on it during any publish pass that includes it. A `publish()` called reentrantly from inside `on_event` will therefore always attempt to re-lock the currently-executing subscriber's own `Mutex` as soon as that subscriber is reached in the reentrant pass — `std::sync::Mutex` is not reentrant, so this deadlocks unconditionally, independent of topic filtering and independent of the `RwLock` fix (a different lock). bones' `Adapter::is_subscribed_to` always returns `true` — all filtering happens after, inside `on_event` — which is exactly the shape most exposed to this hazard.

## Decision

Bones' bus must never call the underlying `publish()` from within a `Handler::handle` call, full stop — not "until pubsub-bus is patched," but permanently, regardless of any future pubsub-bus change. The deferred dispatch queue (ADR-013, rung-1 increment) is the mechanism that guarantees this: handlers only enqueue; the runner's `dispatch` phase is the only caller of the real `publish()`, outside any handler's call stack.

## Rationale

- Making reentrant publish safe at the pubsub-bus layer would require replacing its per-subscriber `Mutex` with a reentrant lock — a materially larger change with its own correctness tradeoffs (a subscriber re-entering its own `on_event` mid-mutation of its fields) — not something to design around opportunistically inside this fix.
- The persistent-adapter workaround genuinely is superseded once real subscriber removal exists upstream (ADR-013's claim holds there). Deferred dispatch is a different kind of fix — it exists to make reentrant synchronous calls structurally impossible, not to route around a missing API — so no upstream change retires it.

## Rejected alternatives

- **Leave ADR-013's wording as-is** — a future reader optimizing away "workarounds" after the pubsub-bus fix lands would remove the one thing preventing a deadlock the moment any real reactive handler exists.
