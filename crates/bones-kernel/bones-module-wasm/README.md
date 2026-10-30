# wasm

The extension host, as an opt-in native module ([ADR-040](../../../docs/adr/ADR-040-the-wasm-extension-host-is-an-opt-in-module.md)). Enabled by `bones-kernel`'s `wasm` feature, which is off by default and on in the shipped `bones` executable. Without it an engine has no extensions and links no WASM runtime; everything else works unchanged.

- Behaviour — the contract, the state machine, faults, hot reload, and runtime activation: [docs/design/extensions.md](../../../docs/design/extensions.md).
- The contract it hosts: [wit/extension.wit](../../../wit/extension.wit).

## What it owns

- **`host`** — loads one WASM component against the `bones:extension` world and calls its exports. Wires in `wasmtime-wasi` with a deny-by-default `WasiCtx` and enforces the per-extension inbound and publish allowances.
- **`loading`** — attaches a component to the bus and the call registry under its own name, and discovers `.wasm` files.
- **`supervisor`** — quarantines a faulted extension (ADR-007), hot-reloads a changed file, and applies runtime `load`, `unload`, and `reload` commands.
- **`lifecycle`** — publishes every state transition on `core/lifecycle`.
- **`Wasm`** — the module that composes them. Its `init` loads the startup extensions; `before_tick` and `after_tick` run the supervisor; `begin_shutdown` stops every extension while the other modules can still hear from them.

## On the bus

The module registers one endpoint per extension, named by its file stem, exactly as any module registers its own. The bus does not know an extension from a module, so an extension reaches every module — first-party or embedder-injected — and every module reaches it, with nothing wired up for extensions specifically.

The module's own endpoint is `extension-manager`, where runtime commands arrive on `core/extensions/*` from the one controller the composition authorizes.

## Composing it

`Kernel`'s extension methods — `extensions_dir`, `catalog_extension`, `startup_extension`, `required_extension`, `extension_controller`, and the budgets — build and register it, after every other module so an extension's own `init` can call any of them. Register it by hand with `.module(Wasm::new(logger)...)` only when composing without those methods, and then register it last for the same reason.
