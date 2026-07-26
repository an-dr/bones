# bus

The message bus (ADR-003, ADR-009, ADR-013): pub/sub over hierarchical
topics with exact and prefix-wildcard subscriptions, built on `pubsub-bus`.

- `Envelope` — topic, sender, payload.
- `Handler` — implemented by anything that receives envelopes.
- `Bus` — register/unregister endpoints, publish (enqueue), dispatch (deliver).
  Cheap to clone; every clone shares the same underlying bus.

Publish only enqueues; delivery happens on `dispatch()`, deferred so a
handler can safely publish in response to what it just received (ADR-015).

`EndpointBudget` supplies ADR-007's per-frame inbound and publish allowances
for untrusted endpoints. A bounded adapter drops matching deliveries over its
allowance; the extension host applies the same object before guest publishes.
`Bus::begin_frame` resets every registered allowance while cumulative
`DropCounters` and the violation flag remain observable; `Runner::step` calls
it at the authoritative frame boundary.

## Direct send (ADR-010)

`Registry` addresses endpoints by name, separately from `Bus`'s by-topic
pub/sub:

- `Respond` — implemented by anything answerable via direct send; returns a
  reply instead of nothing.
- `Registry` — register/unregister named targets; `call(from, to, payload)`
  invokes `to` synchronously and returns its reply. A `to` already in the
  current call chain fails immediately with `SendError::Cycle` rather than
  deadlocking.

`ModuleRegistration` attaches a native `Module` to both message paths at
runtime and owns its shutdown/unregistration. It supports temporary module
compositions without rebuilding the engine.
