# ADR-042: Extension permissions are deny-lists the composition root declares

## Problem

The sandbox limits what an extension can reach in the operating system, not what it may say on the bus. Any extension may publish or subscribe to any topic and `send` to any endpoint, including `os/*`, which opens URLs and file dialogs, and `files`, which reads a granted directory. That suits an application's own extensions. It is too generous for extensions the application did not write, such as third-party plugins, or code an AI agent generated and nobody has reviewed yet.

## Decision

- **An application can deny an extension a set of topics and endpoints.** Entries use the subscription pattern syntax: an exact name (`files`, `gfx/set-display`) or a prefix ending in `*` (`os/*`).
- **Only the composition root declares them**, through the kernel builder or, in the shipped engine, a `[deny]` table in `bones.toml`. An extension never declares its own permissions, because a guard the guarded code writes guards nothing.
- **The extension host enforces them** on the extension's endpoint, as part of the `wasm` module. A denied `publish` is dropped. A denied `subscribe` is ignored, and so is any pattern that could match a denied topic. A denied `send` fails with `unknown-endpoint`, as if the endpoint did not exist. Each refusal is logged once per extension and target.
- **Nothing outside the extension host changes.** The bus, the ABI, and the native modules are untouched, and an extension without a deny-list behaves exactly as before.

## Rationale

The bus is universal by design (ADR-040), so the boundary belongs where an extension meets it, which is the endpoint the host registers for it. A deny-list covers the realistic need: keep a plugin away from a few powerful capabilities while it uses everything else. Answering a denied `send` with `unknown-endpoint` needs no new error case in the WIT package, and it tells the extension nothing about what exists. The log line tells the application developer instead.

## Rejected alternatives

- **Allow-lists.** Safer in principle, but every core topic an ordinary extension needs, such as ticks, input, drawing, and lifecycle, would have to be listed for each plugin. Nobody would keep those lists correct. Deny-lists can grow an allow mode later without changing what exists.
- **A manifest shipped beside the `.wasm`.** It is written by whoever wrote the extension, which is exactly the party the permission exists to constrain.
- **A new `denied` send error in the WIT package.** It would break every extension (ADR-029) to give an untrusted extension information it does not need.
