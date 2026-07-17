# host

Loads a WASM component against the `bones:core` contract (`wit/core.wit`)
and calls its exports. Registers as an ordinary bus `Handler` — on the bus,
the host is indistinguishable from any other endpoint.

- `Host::load` — instantiates the component, links the `log`/`subscribe`/
  `publish`/`send` imports against the given `Bus`/`Registry`, calls `init`.
- `Host::requested_topics` — topics subscribed during `init`; drained once
  by whoever registers the `Host` on the bus.
- `Handler::handle`/`Host::respond` — dispatch to `on-tick`/`on-message`.
  Every call runs under a per-call time budget (ADR-007); a trap or
  timeout marks the `Host` permanently faulted (`Host::is_faulted`).
  Quarantining a faulted `Host` is the caller's job, not this crate's.

Also wires in `wasmtime-wasi` with a deny-by-default `WasiCtx`: any
`wasm32-wasip2` component imports some WASI Preview 2 interfaces via Rust's
std runtime, even without direct WASI use in the guest.
