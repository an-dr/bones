# Task: pong

Build a two-player Pong game as a bones product in this folder.

A bones release is unpacked at `{{engine}}`: the engine, `bones-cli`, the `bones-user` agent plugin in `agent-plugins/`, and the extension ABI in `wit/`. Create the project with:

```sh
{{engine}}/bones-cli new pong --kind game{{bones_path}}
```

You are done when `cargo test` passes in the project and the game below works. Do not ask a person for help; the release's documentation and tools are all you have.

## The contract

The canvas is 800 × 600. Every frame publishes `gfx/clear` and then:

- **Left paddle:** a filled `gfx/draw-rect` 10 wide and 80 high at x = 20. It starts at y = 260. W moves it up and S moves it down, at 300 pixels per second while the key is held. It stays within 0 ≤ y ≤ 520.
- **Right paddle:** the same, at x = 770, moved by the Up and Down arrow keys.
- **Ball:** a filled `gfx/draw-rect` 10 × 10. It starts at x = 395, y = 295, moving at 200 pixels per second right and 150 down. It bounces off the top and bottom edges and off either paddle.
- **Score:** a `gfx/draw-text` reading `<left> - <right>`, starting at `0 - 0`. When the ball leaves past the left edge the right player scores, and past the right edge the left player scores. The ball then restarts from the centre.

No other rectangles are drawn.
