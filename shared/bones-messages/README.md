# bones-messages

Typed core-defined bus messages shared by native engine components and Rust
WASM guests. Each exact topic has a message type implementing the common
`Message`, `EncodeMessage`, and `DecodeMessage` interfaces. Decoding uses one
structured `DecodeError` and may borrow strings or large byte fields from the
payload.

The crate also exposes the fixed-layout little-endian `Reader` and `Writer`
used by those messages. It stays dependency-free so it builds for
`wasm32-wasip2` and native targets. The encoding remains simple enough for
guests in other languages to reproduce without Rust or serde.

- `tick` — `Tick` on `core/tick`.
- `lifecycle` — `LifecycleEvent` and `Event` on `core/lifecycle`.
- `input` — `KeyDown` and `KeyUp` keyboard events.
- `gfx` — `Clear`, `LoadSprite`, `DrawSprite`, `DrawRect`, `DrawLine`,
  `DrawCircle`, `DrawTriangle`, and `DrawText`, plus the wildcard-friendly
  `Command` dispatcher used by the renderer.
- `game_core` — `EntityOp` simulation/presentation operations, tilemap loads,
  and collision events. Existing operation tags and payloads remain stable;
  richer behavior is added with new tags (ADR-023). Its non-wire
  `ObjectFacing` helper classifies 2D velocity into either four cardinal or
  eight octagonal directions for host and guest gameplay code.

This is a standalone workspace, like each `extensions/*` crate, so both the
main workspace and separately built extensions can depend on it.
