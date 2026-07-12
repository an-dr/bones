# host

Loads a WASM component against the `bones:core` contract and calls its
exports. Registers as an ordinary bus `Handler` — on the bus, the host is
indistinguishable from any other endpoint.

- `Host::load` — takes the extension's bus endpoint name and a `Bus` handle,
  instantiates the component, links `log`/`subscribe`/`publish` imports,
  calls `init` once. `publish` forwards straight to that same `Bus`, stamped
  with the extension's name as sender.
- `Host::requested_topics` — topics the extension asked for via `subscribe`
  during `init` (opt-in, including `core/tick` — messaging.md); read once by
  whoever registers the `Host` on the bus.
- Deliveries dispatch to `on-tick` (for `core/tick`) or `on-message` (every
  other subscribed topic). Every call, `init` included, runs under a
  per-call time budget (ADR-007) enforced by wasmtime's epoch interruption;
  `new_engine` spawns the ticker that advances it. A call that traps or
  exceeds its budget marks the `Host` permanently faulted
  (`Host::is_faulted`) — further deliveries are silently ignored rather than
  risking another hang. Quarantining a faulted `Host` (dropping it,
  releasing its bus/registry registrations) is the caller's job, not this
  crate's.

Also wires in `wasmtime-wasi` with a deny-by-default `WasiCtx`: any
`wasm32-wasip2` component imports some WASI Preview 2 interfaces via Rust's
std runtime, even without direct WASI use in the guest.
