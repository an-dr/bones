# host

Loads a WASM component against the `bones:core` contract and calls its
exports. Registers as an ordinary bus `Handler` subscribed to `core/tick` —
on the bus, the host is indistinguishable from any other endpoint.

- `Host::load` — instantiates a component, links its `log` import to a
  `Logger`, calls `init` once.
- `on-tick` traps are logged, not fatal — no watchdog/quarantine yet (rung 5).

Also wires in `wasmtime-wasi` with a deny-by-default `WasiCtx`: any
`wasm32-wasip2` component imports some WASI Preview 2 interfaces via Rust's
std runtime, even without direct WASI use in the guest.
