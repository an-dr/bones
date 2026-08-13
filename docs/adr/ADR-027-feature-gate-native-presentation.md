# ADR-027: Feature-gate the native presentation stack

## Problem

`runner::Engine` supports construction without a window, renderer, or UI, but the runner crate still depended on `platform`, `renderer`, and `ui` unconditionally. Embedders building an always-on headless service therefore compiled SDL and native presentation dependencies they neither configured nor used. That contradicted the kernel rule that a headless build has no presentation modules registered.

## Decision

- Add a default `presentation` runner feature containing the `platform`, `renderer`, and `ui` dependencies.
- Keep the feature enabled by default so existing Bones applications and embedding code retain the current builder API and behavior.
- Make `web` imply `presentation`.
- Allow headless embedders to depend on `runner` with `default-features = false`; window, renderer, and UI builder methods and fields are absent from that build.
- Expose `BuiltEngine::is_headless()` so custom drivers can assert their composition at runtime without inspecting presentation internals.

## Rationale

This makes the existing headless composition a real dependency boundary while preserving source compatibility for normal Bones builds. It also keeps one engine and supervisor implementation for windowed and service embeddings.

## Rejected alternatives

- **Accept unused native dependencies** — simpler, but headless CI and service distributions would still require a presentation toolchain.
- **Create a second headless runner crate** — avoids conditional compilation but duplicates engine construction, supervision, and shutdown behavior.
- **Disable presentation by default** — the cleanest minimal dependency graph, but it would silently remove established APIs from existing Bones consumers.
