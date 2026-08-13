# ADR-024: Runtime-managed extension activation

## Problem

Discovering every `.wasm` file and immediately initializing it prevents an embedder from selecting one level from a packaged catalog. Dropping an extension ad hoc is also unsafe: its bus endpoint, direct-send registration, and application-owned resources need an orderly lifecycle boundary.

## Decision

The extension host separates recursive discovery from activation:

- A catalog records every uniquely named component beneath configured roots. Directories organize a distribution but do not namespace extension names.
- An embedder chooses a startup allow-list. Other catalog entries remain discovered but uninstantiated.
- Typed runtime commands load, unload, or reload a catalog entry by name. Commands complete asynchronously and every state change is published through the lifecycle event stream. Rejected, malformed, or transactionally failed requests are logged because they do not change lifecycle state.
- Loading instantiates the component, runs `init`, then registers it as running. Unloading calls a new `shutdown` export before releasing its bus endpoint, direct-send registration, and instance.
- Messages published by `shutdown` remain queued for normal dispatch, allowing an extension to remove application resources it created. The extension owns that cleanup; the host owns registration cleanup.

The existing load-all builder behavior remains available. Catalog mode and its startup allow-list are explicit builder configuration for applications that need runtime selection.

## Rationale

Discovery without instantiation keeps packaged content inspectable while avoiding side effects from unselected levels. A host-owned manager preserves the watchdog, registry, lifecycle-event, and duplicate-name invariants already enforced for startup and hot reload. An explicit `shutdown` boundary gives extensions one deterministic opportunity to clean up without teaching generic engine modules about application-specific entity or asset ownership.

## Rejected alternatives

- **Load every extension and keep unselected levels dormant** — smaller, but every level still consumes an instance and correctness depends on each one perfectly suppressing initialization side effects.
- **Put selection in a native application shell** — avoids a runtime host API, but makes hot application content depend on embedder-specific Rust and gives the shell privileged lifecycle access unavailable to WASM extensions.
