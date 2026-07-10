# ADR-003: Hybrid messaging — pub/sub topics + direct request/reply

## Problem

Extensions and core components need to exchange messages. A single topology
(only broadcast or only point-to-point) forces awkward patterns on one class of
use cases.

## Decision

The core hosts a message bus with two primitives:

- **Pub/sub topics** — the default; used for input events, tick, draw commands,
  and any broadcast-style extension traffic.
- **Direct request/reply** — point-to-point messages to a named endpoint when
  the sender needs a response.

## Rationale

- Pub/sub keeps extensions loosely coupled and makes hot reload cheap
  (subscriptions simply re-attach).
- Request/reply covers command-style interactions without abusing topics with
  correlation ids.
- The cost is a slightly larger API surface, accepted deliberately.

## Rejected alternatives

- **Topics only** — request/reply must be emulated with reply-topics and
  correlation ids in every extension.
- **Direct addressing only** — couples extensions to each other's names and
  complicates reload/replacement.
