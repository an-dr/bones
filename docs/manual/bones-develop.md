# bones-develop manual

`bones-develop` is the one tool you install to work with bones. It builds the engine, creates and runs products, lets an AI agent drive them, and moves a project to a new release. What each command is for, as goals, is in the [use cases](../use-cases/bones-develop.md); how the tool is built is in its [crate README](../../crates/bones-develop/README.md).

## Install

You need Rust ([rustup.rs](https://rustup.rs)), `git`, a C compiler, and CMake, because the engine is built from source. On Debian or Ubuntu, also install the libraries the engine links — the list `cargo xtask linux-packages` prints in the bones repository:

```sh
sudo apt install pkg-config libasound2-dev libpulse-dev libx11-dev libxext-dev libxrandr-dev \
  libxcursor-dev libxfixes-dev libxi-dev libxss-dev libxtst-dev libxkbcommon-dev libwayland-dev \
  wayland-protocols libdecor-0-dev libegl1-mesa-dev libgl1-mesa-dev libgles2-mesa-dev libdrm-dev \
  libgbm-dev libdbus-1-dev libudev-dev libibus-1.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libssl-dev
```

Then:

```sh
cargo install --git https://github.com/an-dr/bones bones-develop
rustup target add wasm32-wasip2   # extensions build for this target
bones-develop doctor              # says what is still missing, and how to fix it
```

A release's notes give the same command with `--tag`, to install that release's tool. `bones-develop self-update [--tag <tag>]` reinstalls it later.

## Quick start

```sh
bones-develop new pong --kind game   # a project pinned to the newest release
cd pong
bones-develop dev                    # builds the engine on first use, then runs the product
cargo test                           # the headless checks
```

`dev` rebuilds the extension on every save and the running engine reloads it. The first `dev` of a release builds its engine, which takes a few minutes; later runs and other projects on that release reuse it.

## Create a project

`bones-develop new <name> [--kind <kind>] [--tag <tag>]` writes `./<name>`, pinned to the newest release unless `--tag` names another. Its templates, agent files, and ABI pin come from that release, so the project matches its engine.

| Kind | Starts as |
| --- | --- |
| `game` | A square moved by the arrow keys, drawn with `gfx/*` |
| `egui` | A counter with a button, as an egui widget panel |
| `web` | An HTML page in a web panel that talks to the extension in JSON; uses the web-panel build of the engine |
| `embed` | The `game` extension, loaded by the project's own native application instead of the stock engine |

```text
pong/
├── AGENTS.md              rules for an AI agent working here
├── .mcp.json              registers bones-develop mcp with an agent
├── bones.toml             the engine configuration for this product
├── Cargo.toml             a workspace of the crates below
├── extension/             the product: built for wasm32-wasip2
└── checks/                native tests of the built extension, through bones-test
```

`cargo test` at the root runs `checks`; `bones-develop build` builds `extension` and puts it where the engine loads it. The `embed` kind adds `app/`, which `dev` runs instead of the stock engine.

To use the engine as a library in a Rust program you already have, run `bones-develop add [--tag <tag>]` in its crate: it adds `bones-kernel` pinned to the release. [examples/embedding](../../examples/embedding) shows how to compose it.

## Two ways to keep bones in a project

| | On this machine (default) | In the project (`--submodule [path]`) |
| --- | --- | --- |
| Where bones lives | `~/.bones`, shared by every project | a git submodule in the project, `./bones` unless given |
| What the project pins | `v*` and `abi-v*` tags | the submodule's commit |
| Where its engine is built | `~/.bones/engines/<tag>` | the project's `.bones/engine` |
| Suits | several projects on one machine | a project that must build from its own clone: offline, CI, or a team |

`new` and `add` take `--submodule`. A clone of such a project needs `git clone --recursive`; every other command works the same for both.

## Day to day

| Command | Does |
| --- | --- |
| `dev [--engine <path>] [--devtools <port>]` | Builds the extension, runs the project's engine with its `bones.toml` and the devtools channel open (port 8642 unless given), and rebuilds and reloads on every save. The engine log streams to the terminal |
| `build` | Builds the extension and copies it into `.bones/extensions/` |
| `screenshot [--out <file.png>] [--frames <n>] [--press <key>]... [--window]` | Runs the product without a window, presses each key for one frame, renders the given number of frames (1 unless given), and saves what the product shows as a PNG (`screenshot.png` unless given). `--window` captures a visible window instead, web panels included; a product with web panels needs it |
| `doctor` | Checks the machine and the project and names the fix for each problem, including the engine the project uses and whether it is built |

`--engine <path>` or `BONES_ENGINE` runs a different engine than the project's. `dev` refuses to start when the devtools port is taken, usually by another engine, and says to pass `--devtools <port>`. Every command answers `--help`.

## Let an AI agent work on the product

A new project's `.mcp.json` registers `bones-develop mcp`, an MCP server an agent's client starts. Its tools let the agent run, observe, and steer the product without reading engine source, in the message format of the engine the project pins:

| Tool | Does |
| --- | --- |
| `topics` | Lists core topics with their payload layout, optionally under a prefix |
| `launch`, `stop` | Builds the project and runs its engine with devtools on, or stops it. `launch` with `offscreen: true` renders without a display |
| `publish` | Publishes a message: `fields` as JSON for a core topic, or `hex` for any topic |
| `send` | Makes a direct send to an endpoint or extension, as text or hex, and returns the reply |
| `messages` | Recent bus traffic, each core payload decoded into fields |
| `extensions`, `load`, `unload`, `reload` | Extension states, and runtime commands |
| `logs` | Recent log lines, filtered by category and level |
| `frame` | The draw commands of the last rendered frame, as text |
| `screenshot` | What the product shows as an image: the canvas, or with `scope: "window"` the whole visible window. It says which it returned |
| `pause`, `resume`, `step` | Hold the simulation and advance it a number of frames |

Every core topic takes `fields`, such as `{"key": "Left"}` for `input/key-down`. A tagged union takes its tag as a field beside the variant's fields, as in `{"tag": 2, "entity_id": 7}` for a `game-core/entity-op` despawn, and a counted list is a JSON array without its count. Any other topic takes `hex`. The server talks to the channel on `BONES_DEVTOOLS_PORT`, or 8642.

## Move to a new release

`bones-develop upgrade <tag>` moves the project to the release `<tag>`: its pins and ABI line, or its submodule. It also rewrites `AGENTS.md` and `.mcp.json` from that release, so an agent's guidance always describes the engine the project runs. The next `dev` builds the new engine. Commit the result, including the submodule if the project has one.

## Engines on this machine

Engines are built from source and kept in `~/.bones` (`BONES_HOME` moves it):

| Command | Does |
| --- | --- |
| `engine install <tag> [--web]` | Builds the engine of a release ahead of time, for example before working offline |
| `engine list` | Shows the built engines and the releases available |
| `engine remove <tag>` | Deletes a release's engines and source |

Each project runs the engine of its own release, so projects on different releases work side by side. `BONES_REPOSITORY` takes bones from a mirror, a fork, or a local repository instead of GitHub.

## Troubleshooting

- **Start with `bones-develop doctor`.** It lists each missing prerequisite or mismatch with its fix.
- **The engine build fails on Windows in a CMake compiler check.** `BONES_HOME` is inside the system temporary folder (`%TEMP%`), where MSVC's check fails; move it.
- **A new project cannot resolve `bones-wasm-sdk`.** The release's matching `abi-v*` tag is missing from the repository it was taken from.
- **An older project's `.mcp.json` names `bones-cli`.** That is the tool's former name: run `bones-develop upgrade` to a current release, or edit the command to `bones-develop`.
