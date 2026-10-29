# Task: repair

A bones product in this folder, `orbit`, is broken. Its square should leave a short trail and move with the arrow keys. Instead it stops drawing after its first frame and ignores the keyboard. Find out why and fix it.

A bones release is unpacked at `{{engine}}`: the engine, `bones-cli`, the `bones-user` agent plugin in `agent-plugins/`, and the extension ABI in `wit/`. The project is `orbit/`. Use the engine's own tools to find the causes rather than guessing.

You are done when `cargo test` passes in the project and the behaviour below holds. Do not ask a person for help; the release's documentation and tools are all you have.

## The contract

- Every frame publishes `gfx/clear`, the square as a filled 40 × 40 `gfx/draw-rect`, and a trail of at most 100 smaller rectangles behind it.
- The arrow keys move the square at 240 pixels per second while held.
- The extension keeps running however long the engine runs.
