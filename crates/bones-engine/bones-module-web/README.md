# web

Optional native web-panel module ([ADR-006](../../../docs/adr/ADR-006-wry-web-panels.md)): owns panels per extension, bridges bus messages and page JSON, and closes an owner's panels when it faults, reloads, or stops.

- Panel lifecycle, the page bridge, the backend seam, and the detachable presentation: [docs/design/presentation.md](../../../docs/design/presentation.md).
- Message field layouts: [wit/wire-format.md](../../../wit/wire-format.md).

Enable the `wry-backend` feature to construct the wry backend from the application's SDL window, before that window moves into the renderer.
