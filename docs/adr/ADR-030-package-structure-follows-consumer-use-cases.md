# ADR-030: Package structure follows consumer use cases

## Problem

The crates were grouped by internal component boundary, and every one was public. All twelve carried generic unprefixed names — `bus`, `ui`, `web`, `platform`, `logging`, `runner` — so an embedder's `Cargo.toml` collided with any other dependency of the same name, and nothing marked which crates were API and which were implementation. `structure.md` claimed the app had no access an embedder lacks, but nothing stopped `app` reaching into `runner`'s internals.

Extension authors had it worse. Every guest hardcoded `wit_bindgen::generate!({ path: "../../wit" })`, a path that resolves only for code exactly two directories deep inside this repository. An author outside it had no supported way to obtain `core.wit` at all.

## Decision

Group packages by what a consumer is trying to do, so each audience depends on exactly one thing.

- **`bones`** — the engine executable. Keeps its binary-only concerns: config parsing, the Windows console attach, exe-relative path resolution.
- **`bones-engine`** — the sole public library surface, for embedders. Re-exports a curated set rather than globbing, and forwards the presentation feature gates.
- **`bones-wasm-sdk`** — the Rust extension SDK, for guest authors. Carries the WIT, generates the bindings, re-exports `bones-messages`, and offers the in-world UI toolkit behind a `game-ui` feature.
- **`bones-messages`** — the contract both sides encode with. Its own crate, on the ABI version line (ADR-029).
- **`bones:core`, the message wire format, and its conformance vectors** — released artifacts, for extension authors in any other language. Not cargo crates.

The name `bones-extension-*` is reserved for actual shipped extensions — `bones-extension-hello` is the only one — and deliberately not used for the SDK.

**Every crate lives directly under one root-level `crates/` directory**, with no audience subdirectory beneath it. Folder name matches crate name throughout, so a path on disk and a name in `cargo tree` are the same string.

**Inside `bones-engine`, the engine-side crates nest as real Cargo crates**, not as Rust modules of one merged crate:

- `crates/bones-engine/bones-kernel/` — the always-present infrastructure: bus, logging, the host-side WIT bindings, platform, extension loading and supervision, and the frame loop.
- `crates/bones-engine/bones-module-{renderer,ui,audio,game-core,web}/` — the optional native modules, each depending on `bones-kernel` and on no other module.

`bones-engine` itself holds only the composition root: the builder that names concrete module types, and the `BuiltEngine` it produces.

## Rationale

A facade solves the collision problem outright and more cheaply than renaming could: if only `bones-engine` is public, no internal name ever reaches a consumer manifest. It also converts the no-privileged-access rule from a convention into a graph constraint — `cargo tree -p bones` shows `bones-engine` and its binary-only dependencies, and nothing else.

The internal crates were renamed anyway, while pre-1.0 breaking changes are free. The facade makes that rename invisible to consumers today, but publishing to a registry would require unique names for every crate in the graph, so doing it now costs nothing and later costs a second migration.

**No audience directories**, because the guarantees above come from the facade, not from the folder tree. An audience-directory layer would be a second, weaker copy of the same boundary — readable by a contributor browsing the tree, enforced nowhere a build or a dependency graph would catch a violation. Flattening removes the redundant layer and leaves the load-bearing one, and it fixes the folder-name/crate-name mismatch in the same pass rather than touching every path twice.

**Real nested crates rather than modules of one crate**, because two properties are worth the extra manifests. A disabled feature means a `bones-module-*` crate is never compiled, not merely `#[cfg]`'d out of a shared one. And the rule that a module never depends on another module's crate stays something Cargo can see: adding such a dependency means adding a manifest edge, which is visible in review and in `cargo tree`, where `use crate::module_audio::...` inside one merged crate would not be.

Grouping the always-present crates by *presence alone* does not work, and this was measured rather than assumed. The first attempt put the orchestration layer in `bones-kernel` alongside bus and logging, because both are always present. But the orchestration then held `Option<Renderer>` and `Option<Ui>` and depended on `bones-module-renderer`, which depends back on `bones-kernel` — `cargo check` reported the cyclic package dependency directly. "Always present" and "depended on by the optional modules" are different positions in the graph, and only the second belongs at the bottom.

The resolution was not to move the orchestration up but to stop it naming modules at all (ADR-031): once the frame loop drives whatever was registered instead of calling `renderer` and `ui` by name, it depends on no module crate and sits in `bones-kernel` with the rest of the always-present tier. The cycle was a symptom of the direct wiring, not a law about layering.

`bones-messages` cannot be absorbed into the SDK, though it is tempting to give a Rust guest author one package containing everything. The host links it directly, and folding it into a package that runs `wit_bindgen::generate!` would make the engine depend on the guest SDK — inverting the contract relationship. The layering that works puts the contract at the bottom, with host and guest SDK both depending on it sideways and neither on the other. Re-exporting from the SDK gives the author the same single dependency without the inversion.

`game-ui` could be absorbed, and was. It is guest-only and its sole dependency is `bones-messages`, so it belongs inside the SDK behind a feature rather than beside the contract crate.

The generated bindings live in a submodule of the SDK rather than at its crate root. `pub_export_macro` marks the export macros `#[macro_export]`, which hoists them to the crate root, and the sibling `pub use` then collides with them there (`E0255`). This was established by building it before the decision was recorded.

## Rejected alternatives

- **`bones-kernel` for the library.** ADR-011 already uses "kernel" for the always-present tier, as against the optional native modules. Reusing the word for the whole engine library would make it name two scopes in one repository, and that boundary decides the dependency rules.
- **`bones-wit` as a cargo crate for other languages.** A Go or C author cannot `cargo add` anything. Serving that audience requires a plain file and a wire-format description, not a crate.
- **A `bones-extensions` umbrella grouping the messages, UI, and future extensions.** It would file the dual-target `bones-messages` under a guest-only banner, and it collides with the host crate that loads extensions.
- **Giving the bare `bones` name to the library instead of the binary.** The Rust convention where a facade takes the project name is real, but most projects run the engine and write extensions without depending on any Rust crate, so the binary is the common case.
- **Audience subdirectories under `crates/`.** Reintroduces the redundant, unenforced layer one level deeper instead of removing it.
- **Name the guest SDK `bones-extension-sdk`.** That prefix is reserved for shipped extensions; applying it to the SDK would blur the distinction the prefix exists to draw between authoring tools and authored content.
- **Modules-in-one-crate.** Every engine-side crate becomes a Rust module of `bones-engine`. Fewer manifests, and it would dissolve the whole class of facade re-export bookkeeping, since nothing would need re-exporting. Rejected because it trades Cargo-enforced isolation between native modules for internal-visibility discipline, trades true optional compilation for `#[cfg]`-gating, and makes module-to-module coupling invisible in review — a manifest edge is greppable, a `use crate::` is not. Worth revisiting if the manifest cost ever outweighs those three.
- **Breaking the orchestration cycle with a feature-gated dependency.** Giving `bones-kernel` an optional dependency on the native modules. Rejected: cargo rejects a path-dependency cycle regardless of which features gate it, because the manifest edge exists either way.
