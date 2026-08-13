# bones core message wire format

**Version 1.0.0**, matching `bones:core@1.0.0` in [core.wit](core.wit). This document and that file are the two halves of the extension ABI: `core.wit` defines the calls, this defines the bytes those calls carry on core-owned topics.

Written so an extension author in any language can produce and consume core messages without reading Rust. The Rust implementation lives in `bones-messages`; where the two ever disagree, the [conformance vectors](vectors/README.md) decide, because both this document and that crate are checked against them.

## Scope

The bus itself is untyped. A payload is a byte string, and an extension may put anything in one for topics it owns — bones neither inspects nor constrains those (ADR-016). What is specified here is only the set of topics bones itself defines, because those are the ones both sides must agree on.

So there are three kinds of traffic, and only the first is in scope:

- **Core-owned topics** — `core/*`, `input/*`, `gfx/*`, `ui/*`, `audio/*`, `game-core/*`, `web/*`, `renderer/*`, `window/*`, `persistence/*`. Encoded exactly as below.
- **Direct sends to core endpoints** — `persistence`, `files`, `web`. Their request and reply shapes are in [Direct-send endpoints](#direct-send-endpoints).
- **Everything else** — opaque application payloads between extensions. Not specified, deliberately.

## Versioning

This document carries the **ABI version line**, not the engine's (ADR-029). It moves when the encoding of an existing message changes or a core topic is removed — both of which break every extension built against the previous version — and not when the engine's Rust API changes.

Adding a new core topic does not break a decoder that does not subscribe to it, and adding a new variant to a tagged union does not break a decoder that never receives one. Both are additive and do not force a major bump on their own. Removing or renumbering a tag does.

There is no version field in a payload. The version is established out of band, by the `bones:core@<version>` import in the component itself, which the host refuses to instantiate on a mismatch — see [README.md](README.md) for how that check behaves.

## Primitives

Every multi-byte number is **little-endian**, two's complement where signed. There is no alignment or padding anywhere: a field starts at the byte after the previous field ends.

| Notation | Size | Encoding |
| --- | --- | --- |
| `u8` | 1 | unsigned byte |
| `u16` | 2 | unsigned, little-endian |
| `u32` | 4 | unsigned, little-endian |
| `i32` | 4 | signed two's complement, little-endian |
| `f32` | 4 | IEEE 754 binary32, little-endian |
| `bool` | 1 | `0` is false; **any non-zero byte is true**. Encoders write `1` |
| `str(n)` | 2 + n | `u16` byte length, then that many bytes of UTF-8. Not NUL-terminated |
| `blob(n)` | 4 + n | `u32` byte length, then that many raw bytes |
| `rest` | — | every remaining byte of the payload, raw. Only ever the final field |
| `str-rest` | — | every remaining byte, as UTF-8. Only ever the final field |

`str` and `blob` differ only in the width of their length prefix and whether the content is required to be UTF-8. A field is written as `str`/`blob` rather than `rest` precisely when something follows it.

A length prefix counts **bytes, not characters**. A `str` cannot exceed 65535 bytes and a `blob` cannot exceed 4294967295; an encoder that is handed more must fail rather than truncate.

## Framing

A payload is the concatenation of its fields, in the order the topic lists them. There is no envelope, no length prefix on the payload as a whole, no type tag, and no field names — the topic determines the shape completely.

Three rules make that unambiguous:

- **Fixed-shape messages must be consumed exactly.** If any byte remains after the last field, the payload is invalid (`trailing-bytes`). Decoders must check this rather than ignore the tail; it is what catches a version skew that happens to parse.
- **`rest` and `str-rest` may appear only as the final field**, and consume whatever is left, including nothing. A message whose only variable field is last uses `rest`; one with a variable field before another field uses `str` or `blob`.
- **A tagged union leads with a `u8` tag**, and the fields that follow depend on it. An unrecognised tag is invalid (`invalid-tag`), not skippable — a payload does not carry the length of its own variant, so a decoder that does not know the tag cannot find the end of it.

Counted sequences are a `u16` count followed by that many elements back to back, each encoded as its element shape says. There is no separator and no terminator.

## Decoding failures

Four failures are distinguishable, and an implementation should report them separately because they mean different things:

| Failure | Meaning |
| --- | --- |
| `truncated` | the payload ended before a field could be read in full |
| `trailing-bytes` | a fixed-shape payload had bytes left after its final field |
| `invalid-tag` | a tagged union carried a tag the contract does not define |
| `invalid-utf8` | a `str`, `str-rest`, or string field was not valid UTF-8 |

A malformed payload is dropped by the receiver, not reported back to the sender. The bus is at-most-once and has no negative acknowledgement (ADR-009).

## Core topics

### `core/tick`

Published by the engine each frame to `core/tick` subscribers (ADR-004).

| Field | Type | Meaning |
| --- | --- | --- |
| `dt` | `f32` | seconds since the previous tick |

### `core/lifecycle`

Published by the engine when an extension's state changes.

| Field | Type | Meaning |
| --- | --- | --- |
| `event` | `u8` | tag, see below |
| `extension` | `str-rest` | the extension's host-stamped name |

| Tag | Event |
| ---: | --- |
| 0 | `loaded` |
| 1 | `faulted` |
| 2 | `reloading` |
| 3 | `reloaded` |
| 4 | `stopped` |

### `core/extensions/load`, `core/extensions/unload`, `core/extensions/reload`

Runtime activation commands (ADR-024), honoured only from the sender the embedder authorised as the extension controller.

| Field | Type | Meaning |
| --- | --- | --- |
| `extension` | `str-rest` | catalog name to act on |

All three share one shape; the topic is the verb.

### `window/close-requested`

Empty payload, zero bytes. The topic is the whole message.

## `input/*`

### `input/key-down`, `input/key-up`

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | `str-rest` | key name, e.g. `"A"`, `"Space"`, `"Left"` |

### `input/mouse-down`, `input/mouse-up`

| Field | Type | Meaning |
| --- | --- | --- |
| `button` | `u8` | platform button index; 1 is left |
| `x` | `f32` | window coordinate |
| `y` | `f32` | window coordinate |

### `input/mouse-move`

| Field | Type | Meaning |
| --- | --- | --- |
| `x` | `f32` | window coordinate |
| `y` | `f32` | window coordinate |
| `dx` | `f32` | movement since the previous event |
| `dy` | `f32` | movement since the previous event |

### `input/mouse-wheel`

| Field | Type | Meaning |
| --- | --- | --- |
| `x` | `f32` | horizontal scroll |
| `y` | `f32` | vertical scroll |

### `input/gamepad-connected`, `input/gamepad-disconnected`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u32` | gamepad instance id |

### `input/gamepad-button-down`, `input/gamepad-button-up`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u32` | gamepad instance id |
| `button` | `str` | button name |

`button` is `str`, not `str-rest`, because the fixed-shape check still applies after it — a decoder must find no bytes beyond it.

### `input/gamepad-axis`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u32` | gamepad instance id |
| `axis` | `str` | axis name |
| `value` | `f32` | normalised position |

## `gfx/*`

Draw commands an extension publishes and the renderer executes (ADR-002). Colors are four `u8` channels in `r`, `g`, `b`, `a` order, straight (non-premultiplied) alpha. `layer` is a `u8`; the renderer draws layers bottom-up and, within a layer, in arrival order.

### `gfx/clear`

| Field | Type |
| --- | --- |
| `r` | `u8` |
| `g` | `u8` |
| `b` | `u8` |
| `a` | `u8` |

### `gfx/clear-draw-batch`

Empty payload, zero bytes.

### `gfx/load-sprite`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u32` | sprite id this image is addressed by afterwards |
| `png_bytes` | `rest` | a PNG image |

### `gfx/draw-sprite`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u32` | a previously loaded sprite id |
| `dst_x` | `i32` | destination rectangle |
| `dst_y` | `i32` | |
| `dst_w` | `u32` | |
| `dst_h` | `u32` | |
| `src_x` | `i32` | source rectangle within the sprite |
| `src_y` | `i32` | |
| `src_w` | `u32` | |
| `src_h` | `u32` | |
| `layer` | `u8` | |
| `angle` | `f32` | rotation in degrees |
| `flip_h` | `bool` | |
| `flip_v` | `bool` | |
| `tint_r` | `u8` | |
| `tint_g` | `u8` | |
| `tint_b` | `u8` | |
| `tint_a` | `u8` | |

### `gfx/draw-rect`

| Field | Type |
| --- | --- |
| `x` | `i32` |
| `y` | `i32` |
| `w` | `u32` |
| `h` | `u32` |
| `filled` | `bool` |
| `r`, `g`, `b`, `a` | `u8` each |
| `layer` | `u8` |
| `screen_space` | `bool` |

### `gfx/draw-line`

| Field | Type |
| --- | --- |
| `x1` | `i32` |
| `y1` | `i32` |
| `x2` | `i32` |
| `y2` | `i32` |
| `r`, `g`, `b`, `a` | `u8` each |
| `layer` | `u8` |

### `gfx/draw-circle`

| Field | Type |
| --- | --- |
| `x` | `i32` |
| `y` | `i32` |
| `radius` | `u32` |
| `filled` | `bool` |
| `r`, `g`, `b`, `a` | `u8` each |
| `layer` | `u8` |

### `gfx/draw-triangle`

| Field | Type |
| --- | --- |
| `x1` | `i32` |
| `y1` | `i32` |
| `x2` | `i32` |
| `y2` | `i32` |
| `x3` | `i32` |
| `y3` | `i32` |
| `filled` | `bool` |
| `r`, `g`, `b`, `a` | `u8` each |
| `layer` | `u8` |

### `gfx/draw-text`

`text` comes **first** and is length-prefixed, because fixed fields follow it.

| Field | Type | Meaning |
| --- | --- | --- |
| `text` | `str` | |
| `x` | `i32` | |
| `y` | `i32` | |
| `size` | `u16` | point size |
| `r`, `g`, `b`, `a` | `u8` each | |
| `layer` | `u8` | |
| `screen_space` | `bool` | |
| `align` | `u8` | `0` left, `1` center, `2` right |

### `gfx/set-camera`

| Field | Type |
| --- | --- |
| `x` | `f32` |
| `y` | `f32` |
| `zoom` | `f32` |

### `gfx/set-display`

| Field | Type |
| --- | --- |
| `width` | `u32` |
| `height` | `u32` |
| `fullscreen` | `bool` |

## `renderer/*`

Published by the renderer, not to it.

### `renderer/display-changed`, `renderer/logical-canvas`

| Field | Type |
| --- | --- |
| `width` | `u32` |
| `height` | `u32` |

## `ui/*`

### `ui/spec`

A whole widget panel, republished whenever it changes (ADR-005).

| Field | Type | Meaning |
| --- | --- | --- |
| `title` | `str` | |
| `count` | `u16` | number of widgets that follow |
| `widgets` | `count` × widget | back to back, no separator |

Each widget is a tagged union:

| Tag | Variant | Fields |
| ---: | --- | --- |
| 0 | label | `text: str` |
| 1 | text-edit | `id: u32`, `text: str` |
| 2 | button | `id: u32`, `label: str` |

Every widget string is `str`, not `str-rest`, because another widget may follow it.

### `ui/clicked`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u32` | the widget id from the spec |

### `ui/changed`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u32` | the widget id from the spec |
| `text` | `str-rest` | the field's new contents |

## `audio/*`

### `audio/load-sound`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u32` | sound id this clip is addressed by afterwards |
| `bytes` | `rest` | encoded audio |

### `audio/play-sound`, `audio/play-music`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u32` | a previously loaded sound id |
| `volume` | `f32` | `0.0` to `1.0` |

### `audio/set-music-volume`

| Field | Type |
| --- | --- |
| `volume` | `f32` |

### `audio/stop-music`

Empty payload, zero bytes.

## `game-core/*`

### `game-core/entity-op`

One tagged union, the whole payload (ADR-023 — the operation set grows additively, so a decoder must expect tags it does not know).

| Tag | Variant | Fields after the tag |
| ---: | --- | --- |
| 0 | spawn | see below |
| 1 | set-velocity | `entity_id: u32`, `vx: f32`, `vy: f32` |
| 2 | despawn | `entity_id: u32` |
| 3 | set-color | `entity_id: u32`, `r`, `g`, `b`, `a: u8` |
| 4 | set-debug-hitboxes | `enabled: bool` |
| 5 | set-paused | `paused: bool` |
| 6 | set-camera-follow | `entity_id: u32`, `viewport_w: f32`, `viewport_h: f32`, `zoom: f32` |
| 7 | set-sprite | `entity_id: u32`, then a sprite presentation |
| 8 | set-camera-smoothing | `responsiveness: f32` |
| 9 | reset | none |
| 10 | set-sprite-tint | `entity_id: u32`, `r`, `g`, `b`, `a: u8` |

**spawn** (tag 0):

| Field | Type | Meaning |
| --- | --- | --- |
| `entity_id` | `u32` | |
| `x` | `f32` | |
| `y` | `f32` | |
| `has_sprite` | `bool` | whether the sprite fields are meaningful |
| `sprite_id` | `u32` | present regardless; zeroed when `has_sprite` is false |
| `frame_w` | `u32` | |
| `frame_h` | `u32` | |
| `frame_count` | `u32` | |
| `frame_duration` | `f32` | seconds per frame |
| `r`, `g`, `b`, `a` | `u8` each | square color, used when there is no sprite |
| `shape` | `u8` | `0` rect, `1` triangle |
| `collider_half_w` | `f32` | |
| `collider_half_h` | `f32` | |
| `body_kind` | `u8` | `0` dynamic, `1` kinematic, `2` frictionless, `3` fixed |
| `worlds` | `u8` | bit flags: `0b01` rapier2d, `0b10` retro |

The five sprite fields are **always present**, zero-filled when `has_sprite` is false, so the variant has one fixed length rather than two. An encoder must write the zeros; a decoder must read past them.

**sprite presentation** (the tail of tag 7):

| Field | Type |
| --- | --- |
| `sprite_id` | `u32` |
| `frame_w` | `u32` |
| `frame_h` | `u32` |
| `frame_count` | `u32` |
| `frame_duration` | `f32` |
| `frames_per_row` | `u32` |
| `draw_w` | `u32` |
| `draw_h` | `u32` |
| `looping` | `bool` |
| `advance_while_stopped` | `bool` |
| `flip_h` | `bool` |
| `flip_v` | `bool` |

### `game-core/entity-transform`

Authoritative position, published by game-core each frame (ADR-026).

| Field | Type |
| --- | --- |
| `entity_id` | `u32` |
| `x` | `f32` |
| `y` | `f32` |

### `game-core/collision`

| Field | Type |
| --- | --- |
| `entity_id_a` | `u32` |
| `entity_id_b` | `u32` |

### `game-core/load-tilemap`

| Field | Type | Meaning |
| --- | --- | --- |
| `tmx_bytes` | `blob` | a Tiled TMX document |
| `count` | `u16` | number of tileset images that follow |
| `images` | `count` × image | back to back |

Each image:

| Field | Type |
| --- | --- |
| `name` | `str` |
| `sprite_id` | `u32` |
| `png_bytes` | `blob` |

`tmx_bytes` is a `blob` rather than `rest` because the tileset images follow it.

## `web/*`

Published by the web module, not to it — panels are opened by a direct `send` to the `web` endpoint, below.

### `web/panel-opened`, `web/panel-closed`

| Field | Type | Meaning |
| --- | --- | --- |
| `owner` | `str` | the extension that owns the panel |
| `panel` | `str-rest` | owner-local panel id |

### `web/panel-failed`

| Field | Type | Meaning |
| --- | --- | --- |
| `owner` | `str` | |
| `panel` | `str` | |
| `reason` | `str-rest` | human-readable failure |

### `web/page-message`

| Field | Type | Meaning |
| --- | --- | --- |
| `owner` | `str` | |
| `panel` | `str` | |
| `json` | `str-rest` | the page's message, as JSON text |

The JSON is not parsed by the engine. It is carried verbatim between the page and the owning extension.

## Direct-send endpoints

A direct `send` (ADR-010) names an endpoint rather than a topic, and completes within the call with a reply. Three endpoints are core-owned.

### `persistence`

- **Save** — publish to the topic `persistence/save` with the payload being the state bytes, `rest`. The sender's own name selects the slot; an extension cannot write another's.
- **Load** — `send` to `persistence` with an empty payload. The reply is the previously saved bytes, or empty if there is nothing saved.

An empty reply is not distinguishable from an empty saved state. That is deliberate: the caller's next move is the same either way.

### `files`

`send` to `files` with the payload being a path as UTF-8, relative to a directory the embedder granted. The reply is the file's raw bytes, or empty.

Empty covers every failure — no such file, not a file, over the size limit, or a path that resolved outside the granted root — for the same reason as `persistence`. The extension is never told the root and cannot escape it.

### `web`

`send` to `web` with a tagged-union payload:

| Tag | Command | Fields after the tag |
| ---: | --- | --- |
| 0 | open | `panel: str`, `source_kind: u8`, `source: str-rest` |
| 1 | close | `panel: str-rest` |
| 2 | navigate | `panel: str`, `url: str-rest` |
| 3 | send-json | `panel: str`, `json: str-rest` |

`source_kind` is `0` for inline HTML and `1` for a URL; `source` is the HTML text or the URL accordingly.

## Conformance

[vectors/](vectors/README.md) holds machine-readable test vectors: for each topic, a payload as hex and the field values it decodes to. An implementation is conformant when it decodes every vector to the stated values and re-encodes those values to the identical bytes.

The vectors are generated from `bones-messages` and checked against it by its own test suite, so they cannot drift from the engine. Check against the vectors rather than against this prose — the prose is here to be understood, the vectors to be executed.
