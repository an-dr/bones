# web

Optional native web-panel module (ADR-006). The protocol core is independent
of any browser runtime: `Web` owns `(extension sender, panel id)` pairs,
decodes direct commands sent to the `web` endpoint, publishes typed lifecycle
and page events, and closes every panel when its owner faults, reloads, stops,
or the application shuts down.

`Backend` is the small platform boundary. Implementations create/close views,
navigate, deliver extension JSON, and return queued page messages or native
close events from `drain_events`. The wry implementation is feature-gated so
this crate and headless consumers can build without webview system libraries.
