# host

Loads a WASM component against the `bones:core` contract and calls its
exports. Registers as an ordinary bus `Handler` — on the bus, the host is
indistinguishable from any other endpoint.

- `Host::load` — takes the extension's bus endpoint name, a `Bus` handle, and
  a `Registry` (bus::Registry), instantiates the component, links
  `log`/`subscribe`/`publish`/`send` imports, calls `init` once. `publish`
  forwards straight to that same `Bus`, stamped with the extension's name as
  sender; `send` (ADR-010) forwards to that same `Registry`.
- `Host::requested_topics` — topics the extension asked for via `subscribe`
  during `init` (opt-in, including `core/tick` — messaging.md); read once by
  whoever registers the `Host` on the bus.
- Deliveries dispatch to `on-tick` (for `core/tick`) or `on-message` (every
  other subscribed topic). Every call, `init` included, runs under a
  per-call timeout (ADR-007's time budget) enforced by wasmtime's epoch
  interruption; `new_engine` spawns the ticker that advances it. A call that
  traps or exceeds it marks the `Host` permanently faulted
  (`Host::is_faulted`) — further deliveries are silently ignored rather than
  risking another hang. Quarantining a faulted `Host` (dropping it,
  releasing its bus/registry registrations) is the caller's job, not this
  crate's.
- `Host::respond` — answers a direct `send` targeting this extension
  (`bus::Respond`): calls `on-message` with an empty topic (direct messages
  have none) and returns its reply. Same timeout/fault handling as ordinary
  deliveries.

Also wires in `wasmtime-wasi` with a deny-by-default `WasiCtx`: any
`wasm32-wasip2` component imports some WASI Preview 2 interfaces via Rust's
std runtime, even without direct WASI use in the guest.
