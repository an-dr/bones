# ADR-029: The two version lines are the two public surfaces

## Problem

Every crate carried a default `0.1.0` nobody had ever bumped, with no rule saying whether those numbers moved together and nothing that would keep them in sync if they did.

The numbers also could not mean one thing, because bones has two audiences with unrelated compatibility needs. An embedder links the engine and breaks when a Rust API changes. An extension author ships a `.wasm` that outlives the engine build which loaded it, in a language that may not be Rust, and breaks when the guest contract changes. A single version cannot promise both: a renderer patch would force extension authors to re-examine compatibility, and an ABI break could hide behind an unchanged engine version.

## Decision

Two independent version lines, each naming one public surface, and both starting at **1.0.0**.

The **engine line** covers `bones-engine`, the `bones` binary, and every other crate under `crates/` that is not on the ABI line. They move in lockstep through `[workspace.package] version`, inherited by each member. This is what an embedder pins.

The **ABI line** covers the `bones:core` package in `wit/core.wit`, `bones-messages`, and `bones-wasm-sdk`, which is a packaging of exactly that contract for Rust. It moves only when the guest contract changes, and it is deliberately not inherited from `[workspace.package]`. This is what an extension author pins, in any language.

Demos carry no version: examples and the reference extension are `0.0.0` with `publish = false`.

Starting both lines at the same digit does not merge them. A renderer change still leaves the ABI line untouched, and a WIT change still leaves the engine line untouched; the two remain separate fields that move for separate reasons.

## Rationale

The split exists because the surfaces fail independently, and a version is only useful if it predicts a failure.

Lockstep within the engine line is right because those crates are never consumed separately — `bones-engine` is the only public one, and nobody wants one internal crate at 0.4 against another at 0.2. Independent per-crate SemVer would be bookkeeping for a distinction that does not exist.

The ABI line is worth its separate number because wasmtime enforces it. A component built against `bones:core@0.2.0` is refused at instantiation by a `@0.1.0` host, with `component imports instance bones:core/host-api@0.2.0, but a matching implementation was not found in the linker`. That was measured, not assumed, before this ADR was written.

Two properties of that enforcement shape the promise this ADR can make. Matching is **exact, not semver-ranged**: any ABI bump breaks every extension in existence, including a bump that only adds. So the ABI version is far more expensive to move than an ordinary SemVer number, and the standing advice — add a topic and a typed message in `bones-messages` rather than change the WIT — is not style but cost avoidance. And the check is **structural**, so a changed function signature is refused even when the version string did not move; a forgotten bump fails safe rather than mismatching silently.

One limit is worth recording: a component that imports nothing is version-agnostic, because unused imports are stripped before the check applies. Every real extension calls the host API, but a trivial component is not evidence of compatibility.

`bones-messages` sits on the ABI line rather than the engine line even though the host crates link it, because what it encodes is the guest contract. The host is one of its two users, not its owner.

Both lines start at 1.0.0 rather than 0.x because a 0.x number carries its own meaning by convention — that the interface is still experimental and a bump might be treated as compatible when it should not be. That signal no longer matches intent. Both audiences arrive at that point together: an embedder linking `bones-engine` and an extension author pinning `bones:core` are being asked to trust a real release at the same time, so there is no basis to move one line to 1.0 and leave the other behind.

## Rejected alternatives

- **One version for everything.** Simplest to operate, but it makes an ABI break indistinguishable from a renderer patch — the one distinction extension authors need most.
- **Independent SemVer per crate.** The crates.io mainstream, and wrong here: maximum bookkeeping for crates that are never consumed apart, behind a facade that makes their versions invisible anyway.
- **Members unversioned at `0.0.0`, product versioned only by git tag.** Honest while distribution is git-only, but bones treats the library as a first-class product, and this would have to be undone the moment anything is published.
- **CalVer for the binary, SemVer for the library.** A real option, rejected as premature: it adds a third concept before there is a release cadence to justify it.
- **Keep both lines below 1.0 while interfaces move.** This ADR originally said exactly that, and it was right until the interfaces stopped moving. Holding 0.x through a release would tell every consumer the opposite of what the release means.
- **Move only one line to 1.0.0.** Considered taking the engine line first on the theory that one audience might be readier than the other. Rejected: nothing about this release treats embedders and extension authors differently, so nothing justifies giving their version lines different starting confidence.
