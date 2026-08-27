# ADR-034: OS capabilities are an optional module, splittable per capability

## Problem

The sandbox that makes a WASM extension safe to load also denies it the machine ([ADR-001](ADR-001-wasm-component-model.md)). That is correct for most of what an extension does, and wrong for a desktop application whose behaviour lives in extensions: it needs the clipboard, a browser, a file picker, and the network.

[ADR-018](ADR-018-core-2d-presentation-input-and-persistence-gaps.md) established that new capability crossing the extension trust boundary is a decision to record rather than something to infer from the module pattern. It made that argument for persistence. The same argument applies here and had not been made.

Unlike persistence, these capabilities are not one concern. Clipboard access, opening a URL, native dialogs, and HTTPS fetch happen to be grouped because each needs the host, not because they belong together — and they carry very different risks. Network access is not clipboard access.

## Decision

OS capabilities are provided to guests by a native module, on the same trust tier as any other ([ADR-011](ADR-011-native-core-modules.md)): the guest asks over the bus, the module holds the capability, and the sandbox stays intact.

It is **optional and disabled by default**. It is not compiled unless its feature is enabled, and not registered unless the composition asks for it. An engine that grants no OS capability is the default state.

The module is expected to **split into per-capability modules** as it grows. Today one module covers four capabilities; a host that wants a file picker must currently accept a TLS stack with it. When the set grows, or when a consumer needs finer granularity, splitting it is the anticipated move rather than a redesign — the bus protocol is per-capability already, so the split changes composition rather than any guest-visible contract.

## Rationale

Off by default is the honest default for capability. Granting a sandboxed guest reach into the machine is a decision the embedder makes for their application, and a default-on module would make it a decision nobody made. It also keeps a headless or game-only build free of a TLS stack and native dialog libraries it has no use for — smaller binary, smaller attack surface.

The split is named here rather than done because doing it now would be speculative generality: four capabilities behind one feature is not yet painful, and the boundary between "one module" and "four" is cheap to cross later precisely because guests address capabilities individually over the bus. Recording the intent means a later split is executing this decision, not reversing it.

## Rejected alternatives

- **Give guests direct OS access through the host API.** This dissolves the sandbox boundary rather than crossing it deliberately, and every extension inherits the capability whether it needs one or not.
- **Split into four modules now.** Four crates, four features, and four READMEs for a capability set nobody has yet found too coarse. The cost is paid immediately; the benefit is hypothetical.
- **Enable it by default, like audio and game-core.** Those grant no capability across the trust boundary — an extension that draws or plays a sound cannot reach the machine through them. Defaulting a capability module on would silently widen what every shipped engine offers a guest.
