# host

Loads a WASM component against the `bones:core` contract and calls its
exports. Registers as an ordinary bus `Handler` — on the bus, the host is
indistinguishable from any other endpoint.

- `Host::load` — instantiates a component, links `log`/`subscribe` imports,
  calls `init` once.
- `Host::requested_topics` — topics the extension asked for via `subscribe`
  during `init` (opt-in, including `core/tick` — messaging.md); read once by
  whoever registers the `Host` on the bus.
- Deliveries dispatch to `on-tick` (for `core/tick`) or `on-message` (every
  other subscribed topic). Traps are logged, not fatal — no watchdog/
  quarantine yet (rung 5).

Also wires in `wasmtime-wasi` with a deny-by-default `WasiCtx`: any
`wasm32-wasip2` component imports some WASI Preview 2 interfaces via Rust's
std runtime, even without direct WASI use in the guest.
