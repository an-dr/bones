# ADR-031: Native modules reach each other only through services

## Problem

[ADR-017](ADR-017-native-module-trait-and-typed-service-registry.md) defined the `Module` trait and the typed service registry, and [ADR-011](ADR-011-native-core-modules.md) defined native modules as optional and consumer-composed. Two of the five shipped modules never actually arrived that way. `renderer` and `ui` were constructed by `Engine::build` itself, held as typed `Option<Arc<Mutex<Renderer>>>` and `Option<Arc<Mutex<Ui>>>` fields on `BuiltEngine`, subscribed by the builder rather than by their own `init`, and driven by name in the frame loop — `renderer.render()`, then `ui.update(&mut renderer, width, height)`, then `renderer.present()`. `ui` depended on `bones-module-renderer` directly and called five of its inherent methods.

Three consequences followed, none of them intended. An embedder could not substitute a renderer while keeping the shipped ui, because ui named the concrete type; the composition root's own doc comment claimed "no access an embedder lacks" while two modules had exactly that. `BuiltEngine` exposed `Renderer` and `Ui` publicly although `bones-engine` re-exports neither, so an embedder received values of types it could not name in a signature. And the orchestration layer had to sit in `bones-engine` rather than `bones-kernel` to escape a dependency cycle ([ADR-030](ADR-030-package-structure-follows-consumer-use-cases.md)) that existed only because the orchestration named those two module crates.

The same asymmetry ran through input. `ui` alone got first look at raw events, because the loop called `ui.feed_event(event)` by name inside the platform's pre-consumption hook ([ADR-008](ADR-008-layered-input-focus.md)). A `.module(...)`-injected overlay drawing its own interactive widgets had no way to claim a click landing on them.

## Decision

A native module never names another native module's crate. Everything one module needs from another travels through the service registry or the `Module` trait, and the built-in modules use exactly the same routes an injected one does.

Three things follow concretely.

The `draw-target` service exists, in `bones-kernel`: `UiVertex`, `UiMesh`, and a five-method `DrawTarget` trait, with `Box<dyn DrawTarget>` as the registered service type. `renderer` provides it from its own `init`; `ui` consumes it in its own `init` and errors if none was provided. `bones-module-ui` no longer depends on `bones-module-renderer`.

`Module` gains `filter_event`, an optional hook taking the raw platform event and returning whether it claims it, defaulting to claiming nothing. The frame loop offers each event to every module in registration order and stops at the first claim, naming none of them.

`.renderer()` and `.ui()` remain on the builder as sugar over `.module(...)`. They construct the module and hand it to `register_module`, the same function every injected module goes through; `BuiltEngine` holds only `Vec<Arc<Mutex<Box<dyn Module>>>>`, and the render-then-present phase order comes from registration order rather than from named calls.

## Rationale

The service registry was already the answer to this question — `window-surface` had been solving the identical problem between `platform`, `renderer`, and `web` since ADR-017. `draw-target` was named in design/modules.md from the start and simply never built; what the direct wiring bought was one less trait, and what it cost was the substitutability that made native modules worth having as a tier.

Putting `DrawTarget` and the mesh types in `bones-kernel` rather than in either module or a crate of their own follows from what a service is: a contract *between* two optional modules, which neither can own without the consumer depending on the provider. That is the same position `window-surface` occupies, beside the `Module` trait and `ServiceRegistry` that define the mechanism. The types are plain data with no SDL and no rendering backend in them, so the always-present tier learns a shape, not a dependency.

`filter_event` takes `&sdl3::event::Event` rather than a backend-neutral event, and is feature-gated with `platform` for that reason. The neutral `input/*` vocabulary carries neither text input nor modifier state, so a translated event would silently lose what egui needs; inventing that vocabulary is a real decision that deserves its own ADR rather than being folded into this one. The gate matches the feature that provides the event source, so a headless build has no hook and no `sdl3` either way.

Keeping `.renderer()` and `.ui()` as builder sugar preserves the call site exactly — `Engine::new().window(...).renderer().ui().run()` is unchanged — while moving what they do onto the generic path. The composition root may name module types; that is what a composition root is for. What it may no longer do is give the modules it names any capability an injected module lacks.

The dependency cycle that once forced the orchestration up into `bones-engine` was a symptom of the direct wiring, not a law about layering: with the orchestration naming no module crate, `Runner`, `Supervisor`, and extension loading sit in `bones-kernel`, leaving `bones-engine` holding only the builder.

## Rejected alternatives

- **A concrete shared handle instead of a trait.** `renderer` provides `Arc<Mutex<Renderer>>` as the service and `ui` resolves that. Smallest possible change, no new types. Rejected: `ui` stays compile-time bound to the shipped renderer, so substituting a renderer still forces substituting the ui with it — the coupling moves rather than going away.
- **`DrawTarget` in its own crate.** A crate holding the trait and mesh types, depended on by kernel, renderer, and ui. Equally substitutable, and it keeps the kernel strictly free of presentation shapes. Rejected as disproportionate: a whole crate for roughly fifty lines of plain data, and it sets a precedent that every future module-to-module contract gets its own.
- **Meshes over the bus.** `ui` publishes tessellated output as messages and `renderer` subscribes, removing the service question entirely. Rejected on cost and ordering: thousands of vertices per frame would be encoded and decoded at tick rate, and the guarantee that ui draws above every `gfx/*` batch would depend on dispatch order rather than on phase order.
- **A neutral input event for `filter_event`.** `platform` translates SDL once into a kernel-owned event enum, and the `input/*` vocabulary gains text input and modifiers. The right long-term shape, and it would let the hook exist in headless builds. Deferred rather than rejected: it needs a fidelity audit of every event the ui module currently reads, which is its own piece of work.
