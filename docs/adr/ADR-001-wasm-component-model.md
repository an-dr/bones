# ADR-001: WASM Component Model as the extension ABI

## Problem

Extensions must be writable in any language and exchange structured data with
the core and each other. The engine needs one ABI that defines what "any
language" means in practice.

## Decision

Extensions are WASM **components** (WASM Component Model). The contract between
core and extensions is a WIT package; the core embeds wasmtime as the runtime.

## Rationale

- WIT gives typed, language-agnostic interfaces — no hand-rolled pointer/byte
  glue per language.
- Toolchains exist today for Rust, C/C++, Go, Python, and JS.
- It is the forward-looking standard; investing in it avoids a custom-ABI
  migration later.

## Rejected alternatives

- **Core WASM + custom ABI** — works with every toolchain but we would own the
  glue code, serialization, and versioning for each language forever.
- **Core WASM + WASI p1 stdio** — simplest contract, but coarse-grained and
  awkward for interactive/game loops; no typed interfaces.
