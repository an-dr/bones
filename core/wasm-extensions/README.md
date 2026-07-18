# wasm-extensions

Everything concerned with a WASM extension's existence over time, as
opposed to what it can currently *do* (that's `renderer`/`ui`/`audio`):

- **`host`** — loads a WASM component against the `bones:core` contract
  (`wit/core.wit`) and calls its exports. Registers as an ordinary bus
  `Handler`; on the bus, the host is indistinguishable from any other
  endpoint. Also wires in `wasmtime-wasi` with a deny-by-default `WasiCtx`.
- **`lifecycle`** — the topic every extension state transition (Loaded,
  Faulted, Reloading, Reloaded, Stopped) is published on, so tooling and
  other extensions can observe loads, faults, and reloads.
- **`persistence`** — lets an extension save and restore its own state
  across a reload, mediated through the bus instead of a filesystem
  capability grant. Unconditional (`Engine::build` always registers it,
  unlike `audio`/`renderer`/`ui`) — see the module's own doc comment for
  why disabling it wouldn't save anything, and what read-only mode is for
  instead.

See each submodule's doc comment for its own contract.
