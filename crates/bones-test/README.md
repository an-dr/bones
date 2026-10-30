# bones-test

Tests a built extension without a window: load the `.wasm`, send it input, step frames, and check what it published. For a product's own test suite, and for an agent that needs to prove a change works before anyone looks at it.

It runs the real extension host on a headless kernel, so the extension under test runs exactly as it does in the engine. Watchdog, budgets, permissions, and diagnostics all apply. Every message the extension publishes is recorded, including its `gfx/*` draw commands. An extension never draws pixels itself, so its picture is plain data, and a test can compare it as text. When the pixels matter, the `render` feature adds the real renderer in an offscreen window and returns screenshots.

## Use

A product keeps its extension (a `cdylib` built for `wasm32-wasip2`) and its tests (a native crate) side by side, because one crate cannot be both. `bones-cli new` lays them out this way. The test crate depends on this one:

```toml
[dev-dependencies]
bones-test = { git = "https://github.com/an-dr/bones", tag = "v3.1.0" }
```

```rust
use bones_test::App;
use bones_test::messages::gfx::DrawRect;
use bones_test::messages::input::KeyDown;

#[test]
fn the_paddle_moves_up() {
    let wasm = bones_test::build("../extension").unwrap();
    let mut app = App::load(&wasm).unwrap();

    app.step(1);
    let before = app.published::<DrawRect>()[0].y;
    app.clear();

    app.send(&KeyDown { key: "Up" }).step(10);
    assert!(app.published::<DrawRect>().last().unwrap().y < before);
}
```

## Public shape

- `build(dir)` runs `cargo build --release --target wasm32-wasip2` in an extension's directory and returns the path of the `.wasm` it produced.
- `App::load(path)` starts a headless kernel with that one extension, named after the file stem as in the engine. `App::with_kernel(kernel, path)` takes a `Kernel` you configured yourself, for permissions, budgets, or native modules such as `game-core`; `bones_test::kernel` re-exports the kernel crate for that.
- `send(&message)` publishes a core message, as the engine's platform would for input. `send_raw(topic, payload)` does the same for any topic.
- `request(payload)` makes a direct `send` to the extension, as another extension would, and returns its reply.
- `step(frames)` runs that many frames at a fixed 1/60 s each, so a test sees the same thing every run. A message sent before `step` arrives during the first of those frames, before its tick, and what the last frame publishes is delivered before `step` returns.
- `published::<M>()` decodes every recorded message of type `M`, oldest first. `messages()` returns every recorded envelope, from every sender. `clear()` forgets both.
- `frame_text()` is the last frame as text: a `[sender]` line, then that sender's most recent `gfx/*` batch, one `Debug`-formatted command per line, such as `DrawRect(DrawRect { x: 20, … })`. That is what the renderer would composite. Compare it with a snapshot to catch any visual change.
- `is_running()` is `false` once the extension was quarantined.
- `logs()` returns every log line so far, the extension's own included, as `LEVEL category: message`.
- `Stub::new(name).reply(bytes)` is a native module that answers every direct send to `name` with `bytes`, for an extension whose `init` sends a module the test does not compose, such as `web`. Register it with `Kernel::module` before `App::with_kernel`; a clone kept by the test reads `received()`, the `(sender, payload)` of every send.

`published` collects every message since the last `clear()`, across all the frames in between. For one frame's output, `clear()`, `step(1)`, then read.

## Pixels

Draw commands say what an extension asked for. Pixels say what the player sees: whether the sprite loaded, the text fits, and the layers stack as intended. Turn on the `render` feature for them:

```toml
[dev-dependencies]
bones-test = { git = "https://github.com/an-dr/bones", tag = "v3.1.0", features = ["render"] }
```

```rust
let mut app = App::rendered(&wasm, 800, 600).unwrap();
app.step(1);
let shot = app.screenshot().unwrap();
assert_eq!(shot.pixel(0, 0), Some([20, 20, 30, 255]));
shot.save("docs/media/title-screen.png").unwrap();
```

- `App::rendered(path, width, height)` is `App::load` with the engine's renderer drawing into an offscreen window of that size. It needs no display, so it runs in CI.
- `screenshot()` returns the frame as the renderer composites it now, without advancing the simulation. It holds the PNG, its `width` and `height`, and `pixel(x, y)` as `[r, g, b, a]`. `save(path)` writes the PNG, for documentation or for a person to judge a design.
- Rendered `App`s take turns, because SDL runs one window per process: a second waits until the first is dropped. Headless `App`s run alongside them.
- The feature builds SDL from source, as the engine does, so it needs the engine's build prerequisites (`cargo xtask doctor`).

## Constraints

- Persistence is real. `App::load` gives each `App` its own empty save directory, so tests cannot see each other's state. To test a restart, build both `App`s with `App::with_kernel(Kernel::new().saves_dir(dir), path)` and the same `dir`.

- Headless means no renderer, `ui`, `web`, or `audio` module, and `App::rendered` adds only the renderer. An extension that sends to one of their endpoints gets `unknown-endpoint`. An extension that publishes to one of their topics is still recorded.
- `step` drives `BuiltKernel::step` with a fixed `dt`, not wall-clock time. A hung call still trips the watchdog's wall-clock time budget.
- On the engine version line, consumed by git tag like `bones-kernel`.
