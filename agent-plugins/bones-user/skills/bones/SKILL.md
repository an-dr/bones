---
name: bones
description: Build, run, test, and debug a product on the bones engine — a game, tool, or app written as a WebAssembly extension. Use when the repository has bones.toml, an extension/ crate depending on bones-wasm-sdk, or the user mentions bones, its extensions, or bones-cli.
---

# bones

A bones product is a WebAssembly extension the engine loads. The extension never touches the OS and never renders itself. It publishes and receives typed messages on a bus, and the engine does the rest. Read [references/rules.md](references/rules.md) first; its rules apply here. [references/docs.md](references/docs.md) holds the tools' and the contract's full documentation in one file, for when the summaries below are not enough. A project `bones-cli new` wrote carries the same rules as its `AGENTS.md`.

## Start or find the project

- No project yet: `bones-cli new <name> --kind game|egui|web|embed`, then work inside it.
- Existing project: `bones.toml` sits beside `extension/` (the product) and `checks/` (its tests).
- Toolchain trouble: `bones-cli doctor` names what is missing and the command that fixes it.

## Change something

1. Look up the messages you need with the MCP `topics` tool, or in `wit/wire-format.json`. Use the exact topic and field names.
2. Edit `extension/src/lib.rs` with the typed calls: `bus::subscribe::<T>()` in `init`, `bus::publish(&message)`, and `Inbound::decode` in `on_message`.
3. Add or extend a check in `checks/tests/` with `bones_test::App`:
   - `app.send(&message).step(frames)` drives it;
   - `app.published::<T>()` and `app.frame_text()` show what it did;
   - `app.logs()` holds the engine's diagnostics.
4. Run `cargo test`. Fix every failure and every warning the logs show before going on.

## See it run

With the MCP server (`.mcp.json` registers it):

1. `launch` builds and starts the engine with devtools open; `"offscreen": true` keeps it off the screen.
2. `publish` sends input, such as `{"topic":"input/key-down","fields":{"key":"Left"}}`.
3. `pause` holds the simulation, and `step` advances it a set number of frames, so what you read next is repeatable.
4. `frame` returns the draw commands as text, `screenshot` returns the picture, `messages` the decoded traffic, and `logs` the warnings.
5. `stop` closes the engine.

Without MCP, `bones-cli dev` runs the same engine with the devtools channel on port 8642. Its routes are in the `bones-module-devtools` README.

## Look at it

- `bones-cli screenshot --press Space --frames 60 --out shot.png` saves a frame with no window on screen. Open the PNG to judge a layout, or commit it as a documentation image.
- Web panels are child windows, so only a window capture shows them: `bones-cli screenshot --window`, or the MCP `screenshot` tool with `"scope": "window"` on an engine `launch`ed without `offscreen`.
- In a check, enable `bones-test`'s `render` feature: `App::rendered(&wasm, 800, 600)`, then `app.screenshot()?.pixel(x, y)` asserts on what the player sees, and `save(path)` keeps the image.

## When something is wrong

| Symptom | Look at |
| --- | --- |
| Nothing arrives in `on_message` | The subscription in `init`; `logs` for a refused or misspelt topic |
| Nothing is drawn | `frame`: no batch for your extension means `on_tick` did not publish |
| The extension stopped | `extensions` says `faulted`; `logs` names the budget, trap, or timeout |
| A payload is ignored | `logs` for a malformed core payload, which names the topic's section of `wit/wire-format.md` |
| It does not load | `bones-cli doctor` for an ABI mismatch between the extension's pin and the engine |
