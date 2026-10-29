# bones-cli

The developer tools for a product built on bones (ADR-043). They ship in every release archive beside the `bones` engine, which they find as their sibling.

```sh
bones-cli new pong --kind game     # a project that builds and passes its check
cd pong
bones-cli dev                      # build, start the engine, rebuild and hot-reload on every save
cargo test                         # the headless check, through bones-test
```

## Commands

| Command | Does |
| --- | --- |
| `new <name> [--kind game\|egui\|web\|embed] [--bones-path <checkout>]` | Writes a project into `./<name>`. It pins the git tags of this release, or a local bones checkout with `--bones-path` |
| `doctor` | Checks the machine and the project: cargo, the `wasm32-wasip2` target, an engine to run, and that the project's ABI pin matches the engine's ABI. Each failure says how to fix it |
| `build` | Builds the extension for `wasm32-wasip2` and copies it into `.bones/extensions/`, where the project's `bones.toml` points the engine |
| `dev [--engine <path>] [--devtools <port>]` | Builds, starts the engine with the project's `bones.toml` and the devtools channel open (port 8642 unless given), and rebuilds whenever a file under `extension/` changes. The engine hot-reloads the new `.wasm`. Its log streams to the terminal. Ends when the engine exits |
| `screenshot [--out <file.png>] [--frames <n>] [--press <key>]... [--window] [--engine <path>]` | Builds, starts the engine offscreen on a free devtools port, presses each key for one frame, steps the given number of frames (1 unless given), saves the next frame as a PNG (`screenshot.png` unless given), and stops the engine. Nothing appears on screen and no display is needed: for a documentation image, a design check, or a CI job. `--window` runs a visible window instead and captures it whole, web panels included; a project with web panels needs it |
| `mcp` | An MCP server on stdio over the devtools channel, for an AI agent |

`--engine <path>` or `BONES_ENGINE` names the engine when it is not beside `bones-cli`. `dev` refuses to start when the devtools port is already taken, usually by another engine, and says to pass `--devtools <port>`; MCP's `launch` does the same with `BONES_DEVTOOLS_PORT`. Every subcommand answers `--help`.

## MCP

`bones-cli mcp` is an MCP server on stdio, which `.mcp.json` in a new project registers. It turns each devtools route into a tool an agent can call, and uses `wit/wire-format.json` to translate between JSON fields and payload bytes. An agent can then send `{"key": "Left"}` to `input/key-down` without knowing the wire format.

| Tool | Does |
| --- | --- |
| `topics` | Lists core topics with their payload layout, optionally under a prefix: the message contract, looked up rather than guessed |
| `launch`, `stop` | Builds the project and starts its engine with devtools on, or stops it, asking for an orderly exit first. `launch` with `offscreen: true` draws in memory, for a machine without a display |
| `publish` | Publishes a message: `fields` as JSON for a core topic, or `hex` for any topic |
| `send` | Makes a direct send to an endpoint or extension, as text or hex, and returns the reply |
| `messages` | Recent bus traffic, each core payload decoded into fields |
| `extensions`, `load`, `unload`, `reload` | Extension states, and runtime commands |
| `logs` | Recent log lines, filtered by category and level |
| `frame` | The last frame's draw commands as text |
| `screenshot` | The next presented frame as an image: the canvas, or with `scope: "window"` the whole visible window, web panels included. It says which it returned |
| `pause`, `resume`, `step` | Hold the simulation and advance it a number of frames |

It talks to the channel on `BONES_DEVTOOLS_PORT`, or 8642, which is where `launch` and `bones-cli dev` open it. Every core topic takes `fields`. A tagged union takes its tag as a field beside the variant's fields, as in `{"tag": 2, "entity_id": 7}` for a `game-core/entity-op` despawn, and a counted list is a JSON array without its count. Any other topic takes `hex`.

## The project

```text
pong/
├── AGENTS.md              rules for an AI agent working here
├── .mcp.json              registers `bones-cli mcp` with an agent
├── bones.toml             the engine configuration for this product
├── Cargo.toml             a workspace of the two crates below
├── extension/             the product: a cdylib built for wasm32-wasip2
│   └── src/lib.rs
└── checks/                native tests of the built extension, through bones-test
    └── tests/extension.rs
```

The extension and its tests are two crates because one crate cannot be both a WASM component and a native test binary. `cargo test` at the root runs `checks` only; `bones-cli build` builds `extension` only.

The `embed` kind adds `app/`, a native binary that builds its own kernel and loads the extension. `bones-cli dev` then runs `app` instead of the shipped engine.

## Kinds

| Kind | Starts as |
| --- | --- |
| `game` | A square moved by the arrow keys, drawn with `gfx/*` |
| `egui` | A counter with a button, as an egui widget panel |
| `web` | An HTML page in a web panel that talks to the extension in JSON; needs a `-web` engine |
| `embed` | The `game` extension, loaded by the project's own native binary |
