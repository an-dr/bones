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

Enable `wry-backend` to construct `WryBackend` from the application's SDL
window before that window is moved into the renderer. Panels are native child
webviews filling the window's initial client area. Pages send opaque JSON text
to extensions with `window.ipc.postMessage(text)` and receive host messages as
`bones-message` `CustomEvent`s whose `detail` is the same opaque JSON text.
