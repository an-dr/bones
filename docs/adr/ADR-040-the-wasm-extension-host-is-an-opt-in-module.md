# ADR-040: The WASM extension host is an opt-in module

## Problem

[ADR-011](ADR-011-native-core-modules.md) put extension hosting in the kernel infrastructure, the part that cannot be removed. So every engine carries wasmtime and its runtime compiler, even when it runs no extensions at all. The kernel also calls the extension supervisor by name from its frame loop and its shutdown sequence, so the one part that is supposed to name no module names this one.

Not every application built on bones wants extensions. A native application, such as a desktop tool or an Android app built on the library, pays for a runtime it never uses. Android makes the cost concrete: its users install apps rather than drop files into a folder, so a generic binary that loads `.wasm` files has no clear user there.

## Decision

- **The extension host is a native module, `wasm`, in its own crate (`bones-module-wasm`).** It owns loading, dispatch, the watchdog, quarantine, hot reload, runtime activation, lifecycle events, and the host side of `extension.wit`. wasmtime is its dependency, not the kernel's.
- **It is opt-in.** `bones-kernel` builds and runs without it, behind a `wasm` feature that is off by default. The shipped `bones` executable turns it on, so its users see no change. The builder methods that configure extensions remain and are shorthand for registering the module, as `.renderer()` is.
- **The bus stays in the kernel and stays universal.** The module registers one endpoint per extension, exactly as the host did before. An extension reaches every module, first-party or embedder-injected, through the bus, and every module reaches extensions the same way. No module is wired up for extensions specifically.
- **The kernel names no module.** The frame loop and the shutdown sequence call three generic module hooks — before the tick, after the tick, and the start of shutdown — instead of calling the supervisor. Any module may use them.
- **Persistence and files stay in the kernel infrastructure.** They are ordinary bus endpoints that native modules may use as well, so they do not depend on the extension host.
- **The ABI does not change.** `extension.wit` and the wire format stay as they are, and `wit/` stays at the repository root as the published ABI surface (ADR-029). The wire format describes every bus message, native or WASM, so it is not specific to the module.

This supersedes the part of ADR-011 that lists extension hosting in the kernel infrastructure. ADR-011's two-tier split otherwise stands.

## Rationale

**Extensions become what they are best at, by choice.** A sandboxed, hot-reloadable, any-language plugin is the right tool for product logic and third-party plugins. It is the wrong tool for infrastructure an application calls in hot paths, which is better as a native module. Making the host optional lets each application choose, instead of every application paying for both.

**The bus already made this possible.** Extensions and modules were indistinguishable on the bus from the start, and any module can register endpoints. The host only needed to stop being special.

**Users of the binary and authors of extensions are unaffected.** The break lands only on embedders, who must enable the feature to keep extensions. That is an engine-line change (3.0.0); the ABI line does not move (it is at 1.2.0).

## Rejected alternatives

- **Keep the host in the kernel and add a switch to skip loading.** wasmtime would still be compiled and linked, so the cost this decision removes would remain.
- **Move `extension.wit` and the wire format with the module.** Clearer on paper, but it breaks every link and tagged path extension authors already use, and the wire format belongs to the bus rather than to WASM.
- **Make the host a module but on by default in `bones-kernel`.** Embedders who need no extensions would have to discover how to turn it off; opt-in matches every other module.
