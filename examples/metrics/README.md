# metrics

Event-driven data peer for the dashboard example. It publishes a small JSON update on `metrics/updated` twice per second and answers direct `{"get":"history","id":...}` requests with its bounded recent history.

Build and package the component with:

```sh
pwsh build.ps1
```

The script produces `target/wasm32-wasip2/release/metrics.wasm` and a directly runnable `dist/` containing `bones(.exe)`, `bones.toml`, and `extensions/metrics.wasm`. The standalone package demonstrates the publishing peer through logs; the dashboard's package also includes this component automatically for the visual push/pull example.
