# metrics

Event-driven data peer for the dashboard example. It publishes a small JSON
update on `metrics/updated` twice per second and answers direct
`{"get":"history","id":...}` requests with its bounded recent history.

Build the component with:

```sh
cargo build --target wasm32-wasip2 --release
```

Output: `target/wasm32-wasip2/release/metrics.wasm`. The dashboard's package
build includes this component automatically.
