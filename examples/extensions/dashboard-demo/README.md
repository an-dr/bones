# dashboard-demo

WASM owner of the web dashboard example. It embeds `dashboard.html`, opens it as the owner-scoped `main` panel, forwards `metrics/updated` JSON, and relays page history requests to the `metrics` extension with a synchronous send. Page readiness and acknowledgements cross the real wry IPC bridge.

Build the component with:

```sh
cargo build --target wasm32-wasip2 --release
```

Output: `target/wasm32-wasip2/release/dashboard_demo.wasm`.

For the runnable pair:

```sh
pwsh build.ps1
```

This also builds `metrics` and the app with its optional web feature, then assembles `dist/bones(.exe)`, `dist/bones.toml`, and both components. Launch the packaged executable directly; live pushes and page-requested history are visible in the panel.
