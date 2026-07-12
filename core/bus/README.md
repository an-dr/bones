# bus

The message bus (ADR-003, ADR-009, ADR-013): pub/sub over hierarchical
topics with exact and prefix-wildcard subscriptions, built on `pubsub-bus`.

- `Envelope` — topic, sender, correlation, payload.
- `Handler` — implemented by anything that receives envelopes.
- `Bus` — register/unregister endpoints, publish (enqueue), dispatch (deliver).
  Cheap to clone; every clone shares the same underlying bus.

Publish only enqueues; delivery happens on `dispatch()`, deferred so a
handler can safely publish in response to what it just received (ADR-015).
