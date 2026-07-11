# Messaging

Detailed design of the message bus. Decisions: [ADR-003](../adr/ADR-003-hybrid-messaging.md) (hybrid topology), [ADR-009](../adr/ADR-009-delivery-semantics.md) (delivery semantics), [ADR-010](../adr/ADR-010-synchronous-send.md) (synchronous send).

## Message envelope

Every message carries:

| Field       | Meaning                                                            |
| ----------- | ------------------------------------------------------------------ |
| topic       | Hierarchical name (`input/key-down`); absent on direct messages    |
| sender      | Endpoint id of the publisher (extension name or core component)    |
| correlation | Request id linking a reply to its request; direct messages only    |
| payload     | Typed value for core-defined messages; opaque bytes for app-defined |

Core-defined payloads (input, tick, lifecycle, draw commands, widgets, web
panel messages) are strongly typed in the extension contract. Extension-to-
extension payloads are opaque bytes whose schema the participants agree on —
the core never inspects them.

## Topic namespace

| Prefix           | Direction        | Content                                        |
| ---------------- | ---------------- | ---------------------------------------------- |
| `input/*`      | core → ext      | Keyboard, mouse, controller events (post-focus, see [ADR-008](../adr/ADR-008-layered-input-focus.md)) |
| `window/*`     | core → ext      | Window events: resize, close request, DPI      |
| `tray/*`       | core → ext      | Tray icon events: click, menu selection        |
| `core/tick`    | core → ext      | Frame tick with delta time (subscription = opt-in to a frame loop) |
| `core/lifecycle` | core → ext    | Extension state changes: loaded, faulted, reloaded |
| `gfx/*`        | ext → core      | Draw commands for the renderer                 |
| `ui/*`         | both             | Widget specs in, interaction events out        |
| `web/*`        | both             | Panel lifecycle in, JSON frontend messages both ways |
| everything else  | ext ↔ ext       | Application-defined topics                     |

## Pub/sub

- Subscription is by exact topic or prefix wildcard (`input/*`).
- Subscriptions are registered during extension init and released automatically
  on unload/fault — no dangling subscribers.
- Publishing is fire-and-forget for the sender.

## Direct request/reply

- Addressed to an **endpoint** (an extension's registered name, or a core
  service).
- Every request carries a deadline. The outcome is always one of: a reply, an
  error reply (target faulted, reloading, unknown), or a deadline error. No
  request ends in silence.
- **Synchronous** (ADR-010): the caller blocks and the reply is the return
  value of `send`. The target's handler runs as soon as its per-extension
  serialization allows and under its own time budget; the caller's budget
  clock pauses while it waits. Call cycles (A→B→A) fail immediately with an
  error reply.

```mermaid
sequenceDiagram
    participant A as Extension A
    participant Bus as Bus
    participant B as Extension B

    A->>Bus: send(B, request, deadline)
    Bus->>B: on-message (request)
    alt B replies in time
        B-->>Bus: reply
        Bus-->>A: reply (same correlation)
    else deadline exceeded / B unavailable
        Bus-->>A: error reply (same correlation)
    end
```

## Boundary pattern: chunky, not chatty

Every message crossing the bus copies its payload twice (components do not
share memory), so the cost scales with message *count* more than size. Design
extension boundaries around events and coarse data transfers — publish a
snapshot or event stream per frame, send static data once at load — rather
than fine-grained queries on hot paths. Synchronous send (ADR-010) makes
per-frame queries *possible*; this pattern is why they should stay rare.

## Guarantees and limits

- **Ordering:** per-sender FIFO per topic; nothing promised across senders or
  topics (ADR-009).
- **Delivery:** at-most-once; drops happen only toward non-Running extensions
  or over-budget queues, and are counted and logged (ADR-009, ADR-007).
- **Flow control:** bounded inbound queue and per-frame publish allowance per
  extension (ADR-007). Budget violations fault the extension; the bus itself
  never blocks.
