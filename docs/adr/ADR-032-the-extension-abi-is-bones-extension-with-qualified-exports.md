# ADR-032: The extension ABI is bones:extension, with qualified exports

## Problem

The WIT package was `bones:core`, and `world extension` exported four bare functions: `init`, `shutdown`, `on-tick`, `on-message`.

A world-level export takes its **plain name** as the core WebAssembly export symbol. `shutdown` therefore landed in the same flat namespace as POSIX `shutdown(sockfd, how)`, which `libstd` carries for `wasm32-wasip2` socket support, and every guest release build linked with:

```text
rust-lld: function signature mismatch: shutdown
  >>> defined as (i32, i32) -> i32 in ...libstd...
  >>> defined as () -> void in ...the extension...
```

The component's export was wired correctly — extensions shut down properly and the integration tests proved it — so the practical exposure was narrow: a guest calling socket shutdown through `std::net` could reach the wrong symbol. The certain cost was smaller and constant, that every extension author in every language met an unexplained linker warning on their first build of the engine's ABI.

The name was the second problem. `bones:core` says where the package sits, not what it holds, and what it holds is the extension contract: what an extension may call, and what it must provide.

Both were free to fix only until the 1.0 tag. An export name **is** the contract — wasmtime matches imports exactly, refusing instantiation on any difference — so after tagging, either change would invalidate every extension in the field and cost an ABI major.

## Decision

The package is `bones:extension@1.0.0`, declared in `wit/extension.wit`.

The four exports move into an interface named `extension-api`, which the world exports:

```wit
interface extension-api { init; shutdown; on-tick; on-message; }

world extension {
    import host-api;
    export extension-api;
}
```

Guest symbols are therefore `bones:extension/extension-api#<name>`, which can collide with nothing in libc.

The ABI version stays **1.0.0**. Nothing had been tagged when this landed, so this is not a break from anything published — it is the contract taking its final shape before it freezes.

Bus topics keep their `core/*` prefix. Those name the topics the engine owns, which is a separate axis from the WIT package name and is unaffected.

## Rationale

Qualifying the exports fixes the whole class rather than the one name that happened to clash. `init`, `on-tick`, and `on-message` were exposed to exactly the same collision and had simply not met a libc symbol yet; an interface prefix retires that risk permanently instead of waiting to discover the next one.

`extension-api` pairs with the `host-api` interface that already existed: one names what the host provides, the other what the extension provides. The symmetry makes the world readable at a glance, and it avoids `bones:extension/extension`, which a bare `extension` name would have produced.

The rename is worth doing in the same change because both edits rewrite the same import strings across the same files. Splitting them would have meant two migrations for downstream projects instead of one — and after the tag, two ABI majors.

Renaming cost the guests less than expected, which is worth recording as evidence for the facade pattern generally. `bones-wasm-sdk` re-exports the `Guest` trait, so moving it behind an interface changed one line in the SDK and none in the thirteen guests. Only the `host-api` import path, which guests name directly, needed the one-line change each.

## Rejected alternatives

- **Rename only `shutdown`.** Smallest possible fix for the observed warning. Rejected: it leaves the other three exports in the flat namespace, so the same defect returns the first time libc grows a symbol matching one of them, and it costs an ABI change to buy a partial fix.
- **Keep `bones:core` and only qualify the exports.** Fixes the linker warning without touching the name. Rejected as a missed opportunity rather than as wrong: the same files and the same downstream rebuild were already in play, so deferring the rename would have meant paying the migration cost twice.
- **Ship 1.0 with the warning and fix it at the next ABI major.** What the roadmap recorded before this decision. Rejected once the tag became imminent: a warning shipped in a 1.0 ABI is one every future extension author sees, and the fix gets permanently more expensive the moment the tag exists.
- **`bones:abi` as the package name.** Matches the repository's own vocabulary, which calls this the extension ABI throughout. Rejected as a layer name rather than a domain one — WIT convention names packages for what they cover (`wasi:cli`, `wasi:http`), and `bones:extension` reads that way while `bones:abi` describes its own role.
