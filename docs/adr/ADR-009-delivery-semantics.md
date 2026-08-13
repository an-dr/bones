# ADR-009: Bus delivery semantics — per-sender FIFO, at-most-once

## Problem

Extension authors will build on whatever ordering and delivery behavior the bus exhibits. Leaving it unspecified means the accidental behavior becomes the contract.

## Decision

- **Ordering:** messages from one sender on one topic are delivered to each subscriber in publish order (per-sender FIFO). No ordering is promised across different senders or different topics.
- **Delivery:** at-most-once. A message is dropped only when the subscriber is not in the Running state (reloading, faulted, stopped) or its inbound queue budget (ADR-007) is exhausted. Drops are counted and observable via logging.
- **Request/reply:** a direct request carries a deadline; if the target cannot reply in time (busy, faulted, reloading), the sender receives an error reply rather than silence.

## Rationale

- Per-sender FIFO is what draw-command streams and UI event sequences need, and it is cheap to provide.
- At-most-once is honest about hot reload: the gap is real, so the contract says so instead of pretending with unbounded buffers.
- Error replies on deadline keep request/reply total — every request ends.

## Rejected alternatives

- **Guaranteed delivery with buffering and replay** — nicer reload story, but unbounded buffers and replay semantics are heavy machinery to build now; a future ADR can add opt-in buffered topics if a use case demands it.
- **No ordering guarantees** — maximum implementation freedom, but authors could not rely on causality between their own messages; painful for draw commands.
