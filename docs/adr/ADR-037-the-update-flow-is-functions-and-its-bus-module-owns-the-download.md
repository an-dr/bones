# ADR-037: The update flow is functions, and its bus module owns the download

## Problem

`bones-upgrader` provides the parts of self-update — manifests, checksums, version folders, a launcher — but not the flow that strings them together, nor the HTTPS download, nor the convention for where a release lives. Commits wrote all three in its own host; a second application would have written them again.

The flow has two kinds of caller. An application's own `--install` path and its tests call it directly, before any engine exists. A page or a guest asks for it over the bus while the application runs, and must not wait on the network in the frame thread.

## Decision

- **The flow is plain functions in the leaf crate.** `check`, `stage`, and `install` in `bones-upgrader` take a `Fetch`, a source, and a version and return values. No threads, no bus, no HTTP client. `Fetch` remains the crate's one host requirement ([ADR-033](ADR-033-self-update-is-a-crate-and-its-binaries-avoid-installer-words.md)).
- **The release convention lives beside the flow.** `ReleaseSource::GitHub { owner, repo, app }` names the latest release's `<app>-<platform>.json`, where the platform is fixed at compile time; `ReleaseSource::Url` overrides it.
- **The bus adapter is a module, `bones-module-upgrader`, that owns its HTTPS fetch.** It runs each request off the frame thread and answers with a correlated result. It does not borrow `bones-module-os`'s fetch, so it never depends on another module's crate ([ADR-031](ADR-031-native-modules-reach-each-other-only-through-services.md)), and it uses its own limits: a release archive is far larger than anything a guest should fetch through `os/*`.
- **The source is fixed at composition.** A guest asks to check, stage, or install; it never names the URL a release comes from.

## Rationale

**Functions keep the flow usable where an engine is not.** An install runs from `main` before a window exists, and a test wants a fake `Fetch` and a scratch folder, not a bus. A module that held the logic would force both through machinery they do not need.

**The download belongs to the module rather than the leaf crate** because an HTTP client and a TLS stack are exactly what ADR-033 kept out of `bones-upgrader`. An application that updates from a file share, or tests against canned bytes, should not compile one.

**A guest that could choose the manifest URL could make the host stage any binary.** The checksum comes from the same manifest, so it is no defence. Choosing where updates come from is a decision of whoever built the binary — including a user's override, which the application reads from its settings when it composes the engine — the same trust rule [ADR-036](ADR-036-desktop-registration-is-its-own-crate-and-opt-in-module.md) applies to desktop registration.

## Consequences

- An application that changes its update source in its settings sees the change on its next start.
- The module records the running version at start, through the kernel infrastructure's persistence, so it can tell an application it was just updated. It records nothing for a run outside an installed version folder, so a development build never claims to have been updated.
