# bones package restructuring

Idea note, not a decision. Nothing here has been implemented. Written 2026-08-10 against `main` at `1ec6dab`, for discussion before any ADR is drafted.

## The idea

Group bones' packages by **consumer use case** rather than by internal component boundaries, so that each audience depends on exactly one thing, and version each of the two resulting public surfaces on its own line.

Today all 12 workspace crates carry generic unprefixed names (`bus`, `ui`, `web`, `platform`, `logging`, `runner`, …), all sit at a default `0.1.0` nobody has ever bumped, and all are reachable by anyone depending on the repository. There is no public/private distinction, so an embedder's `Cargo.toml` collides with any other dependency named `logging` or `platform`, and nothing prevents reaching past the intended surface into an internal crate.

## Why: the consumer use cases

| # | Use case | Consumes today | Should consume |
| --- | --- | --- | --- |
| A | Embed the engine and inject a custom native module | `runner`, `bus`, `logging`, … by path | `bones-engine` |
| C | Embed the engine with default modules only | same | `bones-engine` |
| D | Embed headless — tests, CI, no window (ADR-014, ADR-027, ADR-028) | same, features off | `bones-engine`, features off |
| B | Write an extension in Rust | hand-copied `core.wit` + `bones-messages` | `bones-wasm-sdk` |
| F | Write a game extension in Rust | + `game-ui` | `bones-wasm-sdk` + feature |
| E | Write an extension in another language | `core.wit` only; payload format undocumented | `bones:core` + a wire-format spec |
| — | Run the engine | the `bones` binary from a distribution | unchanged |
| — | Contribute to bones itself | everything by path | unchanged |

A, C, and D are not separate audiences so much as increasing depth into one surface, which is why one facade serves all three. All three known embedders play the embedder and extension-author roles at once.

## Proposed packages

| Package | Serves | Kind | Notes |
| --- | --- | --- | --- |
| `bones` | running the engine | Rust binary | currently the package `app`, which already produces a `bones` executable; keeps its binary-only deps (`serde`, `toml`, Windows console attach, exe-relative paths) |
| `bones-engine` | A, C, D | Rust library | the sole public library surface; forwards the presentation feature gates; internals become private behind it |
| `bones-wasm-sdk` | B, F | Rust library, `wasm32-wasip2` | carries the WIT, runs `wit_bindgen`, re-exports `bones-messages`; `game-ui` behind a cargo feature |
| `bones-messages` | contract | Rust library, dual-target | stays a separate crate — see the constraint below |
| `bones:core` + wire-format spec | E | released artifacts | not cargo crates; a Go or C author cannot `cargo add` |
| `bones-<name>` (11 internals) | — | Rust libraries | renamed from `bus`, `logging`, `runner`, `contract`, `platform`, `renderer`, `ui`, `audio`, `wasm-extensions`, `game-core`, `web`; private behind `bones-engine` |

The `bones-extension*` naming is reserved for actual shipped extensions such as `hello`, and is deliberately not used for the SDK.

## The two version lines

The central claim: **the two version lines are the two public surfaces.**

- **Engine line** — `bones-engine`, `bones`, and the 11 internals, moving in lockstep through `[workspace.package] version`. This is what an embedder pins.
- **ABI line** — the `bones:core` WIT package and `bones-messages`, moving only when the guest contract changes. This is what an extension author pins, in any language.

Keeping them separate means a renderer patch cannot force an ABI bump, and an ABI break stays visible even when no Rust API moved. That matters because third-party `.wasm` files are binary artifacts that outlive the engine build which loaded them, and hot reload means the engine meets extensions it never compiled against.

## Constraints found while exploring

These are facts about the current repository, not opinions, and each one shapes the design.

**`bones-messages` cannot be absorbed into the SDK.** Eight `core/` crates depend on it: `runner`, `platform`, `renderer`, `ui`, `audio`, `game-core`, `web`, `wasm-extensions`. Folding it into a package that runs `wit_bindgen::generate!` would make the engine depend on the guest SDK, inverting the contract relationship. The layering that works is: contract at the bottom (`wit/` + `bones-messages`), host and guest SDK both depending on it sideways, neither depending on the other. The SDK re-exports it so a Rust extension author still writes one dependency line.

**The wire format is Rust-only.** `bones-messages` has zero dependencies — no serde, no postcard — and uses a hand-rolled `Reader`/`Writer` codec. Dependency-free makes it genuinely re-implementable in any language, but it is specified only as Rust source. A Go or C author gets `core.wit`, can call `publish`/`subscribe`/`send`, and then must reverse-engineer the payload encoding. ADR-001's "extensions in any language" is therefore true for the calls and false for the payloads.

**There is no way to obtain the WIT out of tree.** All 13 bindgen sites — 12 guests plus the host's `crates/bones-contract` — hardcode `path: "../../wit"`, which resolves only for code exactly two directories deep inside this repository. An out-of-tree author has no supported way to get `core.wit` at all, and nothing records which revision they copied.

**"kernel" is already taken.** `core/README.md` splits `core/` into two tiers per ADR-011: *Kernel — always present* (`bus`, `wasm-extensions`, `contract`, `platform`, `runner`, `logging`) and *Native modules — optional, consumer-composed* (`renderer`, `ui`, `audio`, `game-core`, `web`). A `bones-kernel` package covering both tiers would make the word mean two scopes in one repository, and that boundary decides the dependency rules. Hence `bones-engine`.

**`game-ui` does not meet `shared/`'s stated entry requirement.** `shared/README.md` says the directory is for crates depended on by *both* the native host and WASM guest code, and calls that dual target "the entry requirement". `game-ui`'s only consumer in the repository is `examples/game_core_demo`, a guest; nothing in `core/` references it. Absorbing it into `bones-wasm-sdk` resolves this, leaving `shared/` holding only `bones-messages`, which does meet the rule.

**A facade defers the rename question rather than settling it.** Cargo requires every dependency of a published crate to be published, so if bones ever goes to crates.io the internals need unique registry names regardless. While distribution is git dependencies, the facade alone is sufficient.

## Spike findings

Both questions were answered with running code before any ADR was drafted, against wit-bindgen 0.59.0, wasmtime 47.0.3, and rustc 1.97.1.

### Bindings can be generated in a library crate

`bones-wasm-sdk` can carry the WIT, generate the bindings, and let a `cdylib` guest supply the implementation. The working shape is `pub_export_macro: true` plus `default_bindings_module` naming the SDK's own module path, and the generation must happen **inside a submodule** rather than at the crate root.

Generating at the crate root fails with `E0255: the name __export_world_extension_cabi is defined multiple times`, because `#[macro_export]` hoists the macro to the crate root while the sibling `pub use` re-imports it into that same module. A submodule separates the two. The guest then writes `impl Guest for Component` and `bones_wasm_rs::export!(Component)`, with no `path: "../../wit"` anywhere.

A guest built this way produces a valid component. It emits the same `function signature mismatch: shutdown` linker warning that `crates/bones-extension-hello` already emits today, so the pattern introduces no new warning.

### The ABI version is enforced, and enforcement is exact

A host built against `bones:core@0.1.0` was given components built against four variants:

| Guest built against | Result |
| --- | --- |
| `@0.1.0`, identical | instantiates; `init()` runs |
| `@0.2.0`, minor bump | rejected at instantiate |
| `@1.0.0`, major bump | rejected at instantiate |
| `@0.1.0`, one changed `log` signature | rejected at instantiate |

The rejection is loud and specific: `component imports instance bones:core/host-api@0.2.0, but a matching implementation was not found in the linker`.

Three consequences follow, and they are the whole reason this spike had to run first.

**Matching is exact, not semver-ranged.** A `@0.2.0` guest is refused by a `@0.1.0` host even though the interface is otherwise identical. Any ABI version bump therefore breaks *every* existing extension, including bumps that add nothing. This makes the ABI version far more expensive to move than an ordinary semver number, and it is a strong argument for the advice `wit/README.md` already gives: prefer adding a topic and a typed message in `bones-messages`, which needs no ABI change at all.

**Shape changes are caught even without a version change.** The fourth case kept `@0.1.0` and altered one function signature, and instantiation still failed. The check is structural, not a version-string comparison, so a forgotten version bump still fails safe rather than silently mismatching.

**A guest that imports nothing is version-agnostic.** An earlier run of this same experiment appeared to accept all four variants. The guests had empty function bodies, so their imports were stripped from the component before any type check could apply. Only guests that actually call the host API are constrained by the ABI version — which is every real extension, but it means a trivial component is not a valid test of compatibility.

No permanent regression test is committed for this. Doing so would mean maintaining deliberately mismatched fixture `.wasm` files to assert behaviour owned by wasmtime rather than by bones, and the failure is already loud, specific, and immediate.

## Settled

- Lockstep workspace version for the engine crates; independent ABI version line.
- Internal crates renamed `bones-*` now, while pre-1.0 breaking changes are free.
- `bones` names the app binary; `bones-engine` names the library facade.
- `bones-wasm-sdk` is the Rust extension SDK; `bones-extension*` is reserved for extensions.
- `bones-messages` stays a separate crate, re-exported by the SDK.

## Open

- Does `game-ui` get absorbed into `bones-wasm-sdk` behind a feature, or stay its own crate?
- Is a `bones-messages` wire-format specification in scope, and is it released alongside `core.wit`?
- How is the ABI actually distributed — release artifact, `wit-deps`, or an OCI registry via `wkg`?
- Is crates.io publishing ever intended? It is the only thing that makes the internal rename strictly necessary.
- Does the engine stay pre-1.0, where a minor bump means breaking under cargo's rules?
- When `hello` gains siblings, what marks which extensions ship? `dist.ps1` currently builds every directory in `extensions/` into the distribution.
- Is there an MSRV? No `rust-version` is declared anywhere today, and asserting one needs a real toolchain matrix.

## Current state, for reference

- 12 workspace members, every one at `0.1.0`, no `[workspace.package]` inheritance, no `license`/`repository`/`publish` fields.
- `crates/bones-messages` and `shared/game-ui` are excluded from the workspace so they can build for `wasm32-wasip2`; the 12 examples and `crates/bones-extension-hello` each declare their own `[workspace]`.
- `wit/core.wit` declares `package bones:core@0.1.0`.
- `vendor/pubsub-bus` sits at `3.2.0` and is third-party.
- No release tags. The only tag, `compat-main-2026-08-09`, archives the pre-squash history.
- The rename touches 176 crate-path references, 78 of them top-level `use` lines, across 137 Rust files in `core/`.
