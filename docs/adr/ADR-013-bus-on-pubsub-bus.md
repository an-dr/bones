# ADR-013: Bus built on pubsub-bus, with a persistent adapter and deferred dispatch

## Problem

The bus (ADR-003, ADR-009) needs pub/sub delivery with prefix-wildcard
topics, per-sender FIFO ordering, and subscriptions that release cleanly on
extension/module unload — without introducing deadlock risk from reactive
publish chains (a handler publishing in response to a message it received,
which the architecture treats as normal).

`pubsub-bus` (github.com/an-dr/pubsub-bus, MIT, vendored at
`vendor/pubsub-bus`) is a candidate to build on: a Rust crate, `EventBus<C,
TopicId>` with `Subscriber: Send + Sync { is_subscribed_to, on_event }`,
`TopicId: PartialEq + Clone` (a `String` works, prefix matching is the
subscriber's own logic). Two gaps against what's already documented:

- No subscriber-removal API — once added, a subscriber is permanent.
- `publish()` holds a `std::sync::RwLock` read guard across every subscriber
  callback; a handler calling `publish()` again while being called (nested)
  can deadlock — std's docs call recursive read-locking on this type
  unspecified/deadlock-prone.

The crate's owner intends to fix both upstream (add `remove_subscriber()`;
snapshot-then-release the lock before invoking callbacks) but on an
independent timeline from this implementation.

## Decision

Build bones' bus as a wrapper over pubsub-bus, using two patterns that need
**no upstream change** and remain correct once the fixes land (they simply
become unnecessary, not wrong):

- **One persistent adapter per endpoint.** Each module/extension gets a
  single `Arc<Mutex<Adapter>>` registered with pubsub-bus once, for the
  engine instance's lifetime. Subscribe/release mutate the adapter's
  internal active-pattern set (`is_subscribed_to` checks it); release is
  "stop matching," not "remove from the crate." This also gives per-endpoint
  handler serialization for free, from pubsub-bus's own per-subscriber
  `Mutex` — no separate serialization mechanism needed.
- **Deferred publish via the dispatch phase.** Handlers never call
  pubsub-bus's `publish()` directly; they enqueue outgoing envelopes, and
  the runner's `dispatch` phase drains the queue and calls the real
  `publish()` one message at a time, outside any handler call stack. This
  makes the nested-lock case structurally unreachable and matches
  messaging.md's "publishing is fire-and-forget for the sender" more
  literally than a synchronous call would.

The `Envelope` carries all four fields from messaging.md (topic, sender,
correlation, payload) from the start; `correlation` stays unused until
direct send (ADR-010, rung 5). `sender` is the endpoint's registered name
(String), matching the addressing model in messaging.md — not an
auto-assigned id.

## Rationale

- Both patterns are bones-side wrapper logic, not crate patches — rung 1
  does not block on an external repo's release cycle.
- The adapter model reproduces registry-like release semantics (topic →
  active endpoint) even though pubsub-bus itself is a flat predicate-poll
  list underneath — matches messaging.md's auto-release wording without
  needing pubsub-bus to change.
- Deferring publish to one drain point is also where per-sender FIFO
  ordering (ADR-009) is naturally enforced: one queue, drained in
  arrival order, per dispatch pass.

## Rejected alternatives

- **Map bones subscriptions 1:1 to pubsub-bus subscribers, wait for
  `remove_subscriber()` upstream** — blocks rung 1 on a separate repo's
  timeline for no capability rung 1 needs yet (hot reload is rung 5).
- **Call pubsub-bus's `publish()` synchronously from handlers, rely on the
  upstream lock fix landing before any reactive-publish test is written** —
  same external blocking problem, and leaves a deadlock window open for
  every consumer of the crate until that release ships.
