# ADR-012: Logging via an injected sink trait, resolved at kernel construction

## Problem

Logging must be replaceable later (e.g. by a native C-based logger) without touching call sites throughout the kernel. `bus` and `host` both depend on `logging` (structure.md) before any module or extension exists, so the ADR-011 module mechanism — a bus endpoint registered through the runner — cannot host it: the bus itself isn't up yet when the earliest log lines are emitted.

## Decision

Logging is a small `LogSink` trait, implemented by a default synchronous sink and swapped by passing a different implementation into the kernel builder before construction — dependency injection, not a bus endpoint. Call sites use categories/tags (never "topics" — that word stays reserved for bus pub/sub names). No drop counters or buffering in the default sink; add them if a future sink actually needs backpressure.

## Rationale

- Resolves before the bus exists, so it has no circular dependency on the thing it is used to debug.
- A trait swap is exactly what "replace with a C-based lib later" needs in Rust terms (an FFI-backed `LogSink` slots in the same way).
- Keeping "topic" bus-exclusive avoids two similarly-named but differently- behaved concepts colliding in the docs and in extension authors' heads.

## Rejected alternatives

- **Route logging through the module system** — would need a documented carve-out exception to ADR-011's uniform module model for the one module that must exist before the bus does; not worth the special case.
