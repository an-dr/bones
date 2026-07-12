# buffer_rw

Fixed-layout little-endian byte encoding for bus payloads. Deliberately not
serde: extensions are WASM components in any language (ADR-001), so the byte
layout must be reproducible by hand in any of them, not tied to a Rust
serialization framework's own format conventions. No generic trait, no
derive — just two concrete types for the handful of primitives the `gfx/*`
format uses.

- `Writer` — chainable builder: `.u8(v)`, `.u32(v)`, `.i32(v)`, `.bytes(v)`,
  `.finish()` returns the assembled `Vec<u8>`.
- `Reader` — bounds-checked cursor: `.read_u8()`, `.read_u32()`,
  `.read_i32()`, `.read_rest()` (everything remaining), `.finish()` errors if
  bytes are left over. `Error` converts to `String` via `From`, so callers
  that model their own errors as `String` can use `?` directly.

Used by `core/renderer`'s `gfx/*` parsing and `extensions/sprite_demo`'s
payload building; the format itself (documented in `core/renderer/README.md`)
is unchanged from before this crate existed.

Its own standalone workspace (like `extensions/*`) so both the main
workspace and an extension's separate one can depend on it — root
`Cargo.toml` excludes it to avoid a nested-workspace conflict.
