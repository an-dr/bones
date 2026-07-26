# Worked example: "dashboard" — a web panel extension

A status dashboard rendered as a web page: an HTML/JS frontend in a wry panel,
fed with data the extension gathers over the bus. Shows the `web/*` backend
end to end, including a direct request to another extension.

## Setup

1. Manifest: name `dashboard`, version `0.1.0`. The extension package bundles
   its frontend assets (HTML/JS/CSS).
2. `init`: subscribe to `web/*` events for its panel and to the app-defined
   topic `metrics/updated`; directly send `web::Command::Open` to the `web`
   endpoint with the bundled page.

## Steady state

No tick subscription — the extension is fully event-driven and idle between
messages. Data flows in two patterns:

- **Push:** another extension publishes on `metrics/updated`; the dashboard
  forwards a JSON summary to its page.
- **Pull:** the page asks for details; the extension resolves it with a direct
  request to the owning extension.

```mermaid
sequenceDiagram
    participant Page as Panel page (JS)
    participant D as dashboard extension
    participant M as metrics extension

    Page->>D: postMessage {get: "history", id: 7}
    D->>M: send request (deadline)
    M-->>D: reply (history data)
    D->>Page: JSON {history…}
    Note over Page: user closes panel
    Page-->>D: web/panel-closed event
```

The page never touches the bus directly: everything crosses through the
owning extension, which decides what its frontend may see and do
(presentation.md).

## Behavior under engine rules

- **Request outcome is total:** if `metrics` is faulted or slow, the dashboard
  gets an error reply by its deadline (ADR-009) and can show "unavailable"
  instead of hanging its page.
- **Panel ownership:** if the dashboard extension unloads or faults, the core
  closes its panel automatically (presentation.md).
- **Optional feature:** on an engine built without the web feature, the
  direct open command returns `UnknownEndpoint` — the extension can detect it and
  degrade (e.g. log and exit cleanly).
