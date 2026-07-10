# Platform layer

Detailed design of the platform layer — the only component that touches the
OS. Everything it observes becomes bus messages; everything it does is
requested via bus messages.

## Responsibilities

| Area      | Provides                                                        |
| --------- | --------------------------------------------------------------- |
| Window    | One SDL window; `window/*` events (resize, close request, DPI)  |
| Tray icon | Icon + menu owned by core config; `tray/*` events (click, menu selection) |
| Input     | Keyboard, mouse, controller — fed into the focus layers (ADR-008) |
| Timing    | Frame pacing and `core/tick` generation with delta time         |
| Webview hosting | Native parenting/positioning for web panels (ADR-006)     |

Single window for now; multi-window is out of scope per
[architecture.md](../architecture.md) and would arrive via a new ADR.

## Frame loop

The concrete sequence behind the event-loop diagram in
[architecture.md](../architecture.md):

1. **Poll** OS events from SDL.
2. **Route input** through the focus layers (web → egui → `input/*`),
   translate window/tray events onto their topics.
3. **Dispatch** the bus: drain queues into `on-message` calls, route direct
   request/replies. Every call runs under the ADR-007 time budget.
4. **Tick**: when the frame interval elapsed, call `on-tick(dt)` on
   `core/tick` subscribers.
5. **Render**: execute retained gfx batches layer by layer, draw the egui
   output above them, present. Web panels composite themselves as native
   views.

The loop's pace is decoupled from extension behavior by design: a slow or
stuck extension is faulted (ADR-007), never waited for indefinitely.

## Shutdown

A close request (window or tray) is published as an event first — extensions
get the chance to react (confirm, save). Actual shutdown calls `shutdown()` on
every Running extension under the usual time budget, then tears down the
platform. A stuck `shutdown()` is abandoned like any budget violation; the
engine always exits.
