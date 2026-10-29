# Building on bones

This product runs on the bones engine. The product's logic is a WebAssembly extension; the engine owns the window, input, rendering, audio, and saving, and the two talk only through messages on a bus. These rules are what an agent needs and cannot guess.

## The loop

- `bones-cli new <name> --kind game|egui|web|embed` starts a project that builds and passes its checks.
- `bones-cli dev` builds the extension, runs the engine, and hot-reloads on every save.
- `cargo test` builds the extension and runs the headless checks in `checks/` through `bones-test`. `App::request(payload)` makes a direct send to your extension and returns its reply.
- `bones-cli screenshot --press <key> --frames <n> --out <file>.png` builds, runs the engine offscreen, and saves what it draws: for a documentation image or to judge a design. With the `render` feature, `bones-test`'s `App::rendered` and `screenshot()` do the same inside a check, and `pixel(x, y)` reads the result.
- `bones-cli doctor` names anything missing on the machine and how to fix it.
- The `bones` MCP server (`bones-cli mcp`, registered by `.mcp.json`) lets you launch the engine, send input, step frames, and read messages, logs, the frame as text, and a screenshot.

A change is done when `cargo test` passes and includes a check for the behaviour you added. To see the result, take a screenshot or read the frame through MCP rather than asking a person. Compare draw commands in checks; look at pixels when the question is how it looks.

## Rules

1. **Look the contract up.** Every core topic and its payload are in `wit/wire-format.json` (the MCP `topics` tool). The prose is in `wit/wire-format.md`. Do not guess a topic name or a field order.
2. **Use the typed calls.** `bus::publish(&message)`, `bus::subscribe::<Message>()`, and `Inbound::decode(&topic, &payload)` take the topic from the type. Use the raw `publish` and `subscribe` imports only for your own topics or a prefix pattern such as `web/*`. The Rust structs group some wire fields: `r`, `g`, `b`, `a` are one `color: (u8, u8, u8, u8)`, `align` is a `TextAlign`, and string fields borrow as `&str`. `bus::load_state` and `bus::read_file` return `Result<Vec<u8>, SendError>`.
3. **Subscribe in `init`.** Nothing is delivered until you subscribe, and `init` is the only place subscriptions are made. Key names are SDL's (`A`, `Left`, `Space`, `Return`); `wit/wire-format.md` lists the common ones under `input/key-down`.
4. **Draw the whole frame every tick.** The renderer keeps each sender's most recent batch of `gfx/*` commands. Publish `gfx/clear` and every draw in `on_tick`; whatever you stop publishing stays on screen.
5. **An empty topic is a direct `send`.** In `on_message` it has a sender and a payload but no topic, and the `Some(bytes)` you return is the reply. A published message has a topic, and your return value is ignored.
6. **Your extension's name is its `.wasm` file stem.** It shares one namespace with the engine's own endpoints: `renderer`, `ui`, `web`, `audio`, `game-core`, `persistence`, `files`, `os`, `desktop`, `instance`, `upgrader`, `devtools`, and `extension-manager`. Do not reuse one, and do not use `engine`, the sender name lifecycle events are trusted by.
7. **Stay inside the budgets.** By default an extension may publish and receive 1024 messages per frame each. Going over quarantines it, and the log names the limit and the `bones.toml` key that raises it.
8. **There is no OS in the sandbox.** No files, network, clipboard, or threads. Save state with `bus::save_state` and load it with `bus::load_state`. Read granted files with `bus::read_file`. Clipboard, browser, dialogs, and HTTPS go through `os/request` when the application enables the `os` module.
9. **Read the warnings.** The engine reports a malformed core payload, a topic nobody subscribes to, a subscription to a topic that is not a core one but nearly is (with the core topic you probably meant), a refused permission, and the cause of a quarantine. Each comes once per extension and topic, under your extension's name, in `logs` and the terminal. Make your checks fail on them: assert `app.is_running()` and that `app.logs()` holds no `WARN` or `ERROR` line.
10. **Keep the pins matched.** The extension depends on `bones-wasm-sdk` at an `abi-v*` tag, and the engine refuses a component built for another ABI. `bones-cli doctor` compares the two. Web panels need a `-web` engine and `web = true` in `bones.toml`.
