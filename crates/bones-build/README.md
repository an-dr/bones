# bones-build

Builds a bones extension for `wasm32-wasip2` and finds the `.wasm` it produced. `cargo xtask`, `bones-cli`, and `bones-test` all build extensions; this crate is the one place that knows how (ADR-043).

## Public shape

- `Build::new(dir)` describes a build of the extension crate in `dir`: release mode, target `wasm32-wasip2`.
  - `.package(name)` selects one package of a workspace.
  - `.show_diagnostics()` streams cargo's errors to the terminal; without it they are captured and returned in the error.
  - `.run()` returns the path of the built `.wasm`.
- `wasm_artifact(messages)` finds the last `.wasm` among the JSON messages `cargo build --message-format=json` prints. Reading the path from cargo, rather than assuming `target/`, keeps a `CARGO_TARGET_DIR` elsewhere working.
- `target_installed(target)` asks rustup whether a target is installed. `INSTALL_TARGET` is the command that installs `wasm32-wasip2`.

## Constraints

It depends on no bones crate and links nothing but `serde_json`. The cargo it runs is `$CARGO` when set, as it is under `cargo test`, and `cargo` from `PATH` otherwise.
