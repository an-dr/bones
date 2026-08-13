# app

The engine executable (structure.md): the default composition most projects run as-is, writing only WASM extensions. Built solely on `bones-engine`'s public surface (ADR-011, ADR-030) — no access an embedder using the same crate lacks, which its dependency list now enforces rather than merely asserts. Package and built binary are both named `bones`.

Opens one window (800x600 by default), feeds keyboard events onto `input/*`, and renders `gfx/*` draw commands (`crates/bones-engine/bones-module-renderer`).

A release build is a windowed Windows binary, so no console appears behind the window; launched from a terminal it attaches to that terminal's console, so log output and a fatal startup error still land where the operator can read them. Debug builds are console binaries throughout. Drop `.wasm` extensions into an `extensions/` directory next to the executable file itself (not the shell's current directory — the binary can be launched from anywhere and still finds its own `extensions/`); `cargo run -p bones` (or the built binary) discovers and runs them.

## Configuration

Reads `bones.toml` next to the executable file, if present — the same exe-relative resolution applies to `extensions_dir` and `saves_dir`, so a `dist/` build stays self-contained regardless of what directory it's launched from. Every field is optional and defaults to the values below; an unknown field or invalid TOML is a startup error rather than a silent no-op.

Set `BONES_CONFIG=<path>` to read the config from somewhere else entirely — `extensions_dir`/`saves_dir` then resolve against *that* file's own directory instead of the exe's. For an embedder running `cargo run -p bones` straight against a vendored `bones` checkout (exe buried in a `target/` directory, no `dist/` involved), this points the engine at a project-root `bones.toml` and its own `extensions/` without copying anything there first.

```toml
extensions_dir = "extensions"
window_title = "bones"
window_width = 800
window_height = 600
renderer = true
web = false
extension_max_inbound = 1024
extension_max_publishes = 1024
```

Web panels require both the cargo feature and configuration switch: `cargo run -p bones --features web` with `web = true`. Ordinary builds keep the wry/WebView2/WKWebView/WebKitGTK dependency out entirely.

The two extension limits are per frame and apply independently to every WASM component. Exceeding either allowance drops and counts the excess work, then quarantines only that component.

For a self-contained build (binary + every extension in one directory), run `pwsh dist.ps1` from the repo root — assembles `dist/bones(.exe)` and `dist/extensions/`, copyable anywhere and run as-is.
