# ADR-016: Typed core messages over the byte-oriented bus

## Problem

Core-defined bus messages share a wire encoding but expose inconsistent Rust interfaces: tick uses free `encode_dt`/`decode_dt` functions, graphics uses command-specific encoders plus `parse_command`, lifecycle owns another codec, and input publishes raw strings. The `bones-vocab` name also obscures that the crate defines messages in both event and command directions.

## Decision

Rename `bones-vocab` to `bones-messages`. Model each exact core-defined topic as a typed message implementing shared `Message`, `EncodeMessage`, and `DecodeMessage` traits. All payload decoding returns one structured `DecodeError`, and payloads containing large byte or string fields may borrow from their input. Topic-family dispatchers may wrap the exact message types.

Keep the bus envelope and extension ABI byte-oriented. Typed messages are a shared host/guest layer over the raw bus, not a closed set enforced by the bus.

## Rationale

One interface makes core messages predictable and enables generic helpers while retaining the open topic namespace required for application-defined messages. Borrowed decoding avoids unnecessary copies for assets and other large payloads. `bones-messages` covers events, commands, and future request/reply payloads more accurately than either `bones-events` or `bones-vocab`.

## Rejected alternatives

- `bones-events`: graphics and future UI traffic include commands, not only events.
- Inherent methods without traits: discoverable, but unable to support generic typed helpers consistently.
- Renaming and normalizing free functions only: leaves payloads fragmented and does not establish a common contract.
- A global core-message enum or typed bus: closes the topic space and couples independent modules to every message family.
