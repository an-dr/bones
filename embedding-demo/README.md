# embedding-demo

Roadmap rung: "Full builder API: custom native-module injection, embedding
demo" — a parent project injecting a custom module and building its own
engine binary (design/modules.md, ADR-017).

A separate `[workspace]` (like `extensions/*`), depending on bones's crates
by relative path the same way a real embedder would depend on them as a git
subrepo or path dependency — nothing here uses anything an external
embedder couldn't. `Clock` is a native module (`bus::Module` +
`bus::Handler`) that subscribes to `core/tick` in its own `init` and logs
the wall-clock time once a second; `main.rs` injects it with `.module(...)`
and runs headlessly (no window, no extensions).

## Run

```sh
cargo run
```

Requires a C toolchain and CMake (transitively, via `platform`'s SDL
dependency) — see the repo root's `dist.ps1` if `cmake` can't find a
generator; set `CMAKE_GENERATOR=Ninja` if one is installed.
