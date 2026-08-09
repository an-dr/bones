# extensions

WASM extensions that ship with a bones distribution. Exactly one lives here:

- [hello](hello/README.md) — the reference extension, and the
  write-your-first-extension tutorial.

[dist.ps1](../dist.ps1) builds every directory here into `dist/extensions/`,
which is why the bar for adding one is high: a distribution is the engine plus
its reference extension, not a demo bundle.

## What belongs here

An extension that a fresh install should ship with. Anything that exists to
demonstrate a capability belongs in [examples/](../examples/README.md)
instead — that is where the other eleven live.

## Adding your own

You do not need to put anything here. Build your extension anywhere and drop
the `.wasm` into the `extensions/` directory beside the engine binary at
runtime; it loads on start. The `extensions_dir` key in `bones.toml` controls
where the engine looks.

Each extension is its own `[workspace]`, built for `wasm32-wasip2` by its own
`build.ps1`, and depends only on [`wit/`](../wit/README.md) and optionally
[`shared/bones-messages`](../shared/bones-messages/README.md) — never on core
crates.
