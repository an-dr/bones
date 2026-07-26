# app

The engine executable (structure.md): the default composition most projects
run as-is, writing only WASM extensions. Built solely on `runner`'s public
builder API (ADR-011) — no access an embedder using the same API lacks.
Package is `app`, built binary is `bones`.

Opens one window (800x600 by default), feeds keyboard events onto `input/*`,
and renders `gfx/*` draw commands (`core/renderer`). Drop `.wasm` extensions
into an `extensions/` directory next to the executable file itself (not the
shell's current directory — the binary can be launched from anywhere and
still finds its own `extensions/`); `cargo run -p app` (or the built binary)
discovers and runs them.

## Configuration

Reads `bones.toml` next to the executable file, if present — the same
exe-relative resolution applies to `extensions_dir` and `saves_dir`, so a
`dist/` build stays self-contained regardless of what directory it's
launched from. Every field is optional and defaults to the values below; an
unknown field or invalid TOML is a startup error rather than a silent no-op.

Set `BONES_CONFIG=<path>` to read the config from somewhere else entirely —
`extensions_dir`/`saves_dir` then resolve against *that* file's own
directory instead of the exe's. For an embedder running `cargo run -p app`
straight against a vendored `bones` checkout (exe buried in a `target/`
directory, no `dist/` involved), this points the engine at a project-root
`bones.toml` and its own `extensions/` without copying anything there first.

```toml
extensions_dir = "extensions"
window_title = "bones"
window_width = 800
window_height = 600
renderer = true
extension_max_inbound = 1024
extension_max_publishes = 1024
```

The two extension limits are per frame and apply independently to every WASM
component. Exceeding either allowance drops and counts the excess work, then
quarantines only that component.

For a self-contained build (binary + every extension in one directory), run
`pwsh dist.ps1` from the repo root — assembles `dist/bones(.exe)` and
`dist/extensions/`, copyable anywhere and run as-is.
