# web

Optional native web-panel module (ADR-006). The protocol core is independent of any browser runtime: `Web` owns `(extension sender, panel id)` pairs, decodes direct commands sent to the `web` endpoint, publishes typed lifecycle and page events, and closes every panel when its owner faults, reloads, stops, or the application shuts down.

`Backend` is the small platform boundary. Implementations create/close views, navigate, deliver extension JSON, update native state once per frame, and return queued page messages or native close events from `drain_events`. The wry implementation is feature-gated so this crate and headless consumers can build without webview system libraries.

Enable `wry-backend` to construct `WryBackend` from the application's SDL window before that window is moved into the renderer. Panels are native child webviews that track the resizable window's live client area. Pages send opaque JSON text to extensions with `window.ipc.postMessage(text)` and receive host messages as `bones-message` `CustomEvent`s whose `detail` is the same opaque JSON text.

An `Html` panel is served over the `bones://` custom protocol rather than set as a document string, so the page loads at a real origin and can use storage and anything else subject to a same-origin check. A `Url` panel navigates as given.

`WryPresentation` is the detachable composition for service embedders. It opens its own SDL parent window, attaches the same `web` endpoint to an existing bus/registry, and fully unregisters it on close. The engine that owns that bus can remain headless and open a new presentation later.
