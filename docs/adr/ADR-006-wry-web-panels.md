# ADR-006: Web UI via wry web panels (optional feature)

## Problem

Some applications want rich, web-based UI beyond what a widget toolkit offers.
Tauri is the established Rust answer, but Tauri-the-framework owns the window
and event loop — which bones' SDL platform layer already owns.

## Decision

Take only Tauri's webview layer: the core optionally embeds **wry** (OS
webview: WebView2 / WKWebView / WebKitGTK). The platform layer gains a "web
panel" surface — a webview child of the SDL window where supported, or a
separate top-level window (the pragmatic fallback on Linux, where WebKitGTK
wants a GTK main loop). Extensions manage panels and exchange JSON with their
web frontends over `web/*` topics; inside the panel, JS `postMessage` bridges
to the bus.

The whole layer sits behind a cargo feature flag so a minimal engine build
carries no webview dependency.

## Rationale

- OS webview, nothing bundled — stays light.
- The bus bridge makes a web panel just another endpoint; no toolkit details
  cross the WASM boundary (consistent with ADR-002/ADR-005).
- Composes with a future out-of-process frontend: same `web/*` vocabulary over
  a network transport.

## Rejected alternatives

- **Full Tauri in-process** — its framework value (config, bundling, updater,
  IPC permissions) assumes it is the app; it fights SDL for the event loop.
- **Separate Tauri app talking to the bus over WebSocket** — clean and still
  possible later, but ships a second process and adds a network transport now.
- **HTTP server + external browser** — simplest, but no native window or tray
  integration; weak as a product experience.
