# app

The engine executable (structure.md): the default composition most projects
run as-is, writing only WASM extensions. Built solely on `runner`'s public
builder API (ADR-011) — no access an embedder using the same API lacks.
Package is `app`, built binary is `bones`.

Opens one 800x600 window (`core/platform`), feeds keyboard events onto
`input/*`, and renders `gfx/*` draw commands (`core/renderer`). Drop `.wasm`
extensions into an `extensions/` directory next to where you run it;
`cargo run -p app` (or the built binary) discovers and runs them.

For a self-contained build (binary + every extension in one directory), run
`pwsh dist.ps1` from the repo root — assembles `dist/bones(.exe)` and
`dist/extensions/`, copyable anywhere and run as-is.
