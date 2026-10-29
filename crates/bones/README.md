# app

The engine executable (architecture/structure.md): the default composition most projects run as-is, writing only WASM extensions. Built solely on `bones-kernel`'s public surface (ADR-011, ADR-030) — no access an embedder using the same crate lacks, which its dependency list now enforces rather than merely asserts. Package and built binary are both named `bones`.

Opens one window (800x600 by default), feeds keyboard events onto `input/*`, and renders `gfx/*` draw commands (`crates/bones-kernel/bones-module-renderer`).

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

`extension_load_timeout_ms` (default 1000) bounds an extension's instantiation and `init`, and `extension_call_timeout_ms` (default 50) bounds each later call. Raise the load timeout for a component that carries its own language runtime, such as a JavaScript one, whose first load can take more than a second.

The two extension limits are per frame and apply independently to every WASM component. Exceeding either allowance drops and counts the excess work, then quarantines only that component.

A `[deny]` table keeps named extensions away from topics and endpoints, which is how to run a plugin you did not write ([design/extensions.md](../../docs/design/extensions.md#permissions)). Each key is an extension's name, the stem of its `.wasm` file:

```toml
[deny]
untrusted_plugin = ["os/*", "files"]
```

Set `BONES_DEVTOOLS_PORT=<port>` to open the loopback devtools channel, which lets a test or an agent drive the running engine and read back what happened: publish any message, read the bus traffic and the log, see each extension's state, and reload one ([bones-module-devtools](../bones-kernel/bones-module-devtools/README.md)). It is unauthenticated and listens on `127.0.0.1` only. Never set it in a shipped launch.

Set `BONES_OFFSCREEN=1` to open the window in memory instead of on a display, through SDL's offscreen video driver. Everything runs as usual, and the devtools `/screenshot` route returns what the renderer drew; `bones-cli screenshot` uses it. Nothing appears on screen, and no display is needed.

Set `BONES_LOG_LEVEL` to `debug`, `info`, `warn`, or `error` to choose the quietest level written; the default is `info`. At `debug` the runner logs every tick.

Set `BONES_LOG_FORMAT=json` to write the log as one JSON object per line, `{"level":"warn","category":"engine","message":"…"}`, for a tool or an agent to parse. Any other value, or none, keeps the plain `[LEVEL] category: message` lines.

For a self-contained build (binary, ABI, and an empty extensions folder), run `cargo xtask dist` from the repo root — assembles `dist/bones(.exe)` and `dist/extensions/`, copyable anywhere and run as-is.
