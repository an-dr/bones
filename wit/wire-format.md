# bones core message wire format

<!-- Generated from wire-format.json by `cargo xtask wire-format`. Do not edit. -->

This specification and [extension.wit](extension.wit) are the two halves of the extension ABI: `extension.wit` defines the calls, this defines the bytes those calls carry on core-owned topics. It describes the ABI line of the `abi-v*` tag or release archive you took it from; `bones-messages` carries the same version number.

Written so an extension author in any language can produce and consume core messages without reading Rust. The Rust implementation lives in `bones-messages`; where the two ever disagree, the [conformance vectors](vectors/README.md) decide, because both this specification and that crate are checked against them.

## Scope

The bus itself is untyped. A payload is a byte string, and an extension may put anything in one for topics it owns — bones neither inspects nor constrains those (ADR-016). What is specified here is only the set of topics bones itself defines, because those are the ones both sides must agree on.

So there are three kinds of traffic, and only the first is in scope:

- **Core-owned topics** — `core/*`, `input/*`, `gfx/*`, `ui/*`, `audio/*`, `game-core/*`, `web/*`, `renderer/*`, `window/*`, `persistence/*`, `instance/*`, `desktop/*`, `os/*`, `upgrader/*`. Encoded exactly as below.
- **Direct sends to core endpoints** — `persistence`, `files`, `web`. Their request and reply shapes are in [Direct-send endpoints](#direct-send-endpoints).
- **Everything else** — opaque application payloads between extensions. Not specified, deliberately.

## Versioning

This specification carries the **ABI version line**, not the engine's (ADR-029). It moves when the encoding of an existing message changes or a core topic is removed — both of which break every extension built against the previous version — and not when the engine's Rust API changes.

Adding a new core topic does not break a decoder that does not subscribe to it, and adding a new variant to a tagged union does not break a decoder that never receives one. Both are additive and do not force a major bump on their own. Removing or renumbering a tag does.

There is no version field in a payload. The version is established out of band, by the `bones:extension@<version>` import in the component itself, which the host refuses to instantiate on a mismatch — see [README.md](README.md) for how that check behaves.

## Primitives

Every multi-byte number is **little-endian**, two's complement where signed. There is no alignment or padding anywhere: a field starts at the byte after the previous field ends.

| Type | Size | Encoding |
| --- | --- | --- |
| `u8` | 1 | unsigned byte |
| `u16` | 2 | unsigned, little-endian |
| `u32` | 4 | unsigned, little-endian |
| `i32` | 4 | signed two's complement, little-endian |
| `f32` | 4 | IEEE 754 binary32, little-endian |
| `bool` | 1 | `0` is false; **any non-zero byte is true**. Encoders write `1` |
| `str` | 2 + n | `u16` byte length, then that many bytes of UTF-8. Not NUL-terminated |
| `blob` | 4 + n | `u32` byte length, then that many raw bytes |
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

The extension host checks every core-topic payload an extension publishes against this specification before delivering it. When one fails, it logs a warning naming the extension, the topic, the failure, the payload's length and leading bytes, and this specification's section for the topic. It warns once per extension and topic, so a malformed message published every frame produces one line. The payload is still delivered, and its receiver drops it as above.

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
| `event` | `u8` | what happened: `0` loaded, `1` faulted, `2` reloading, `3` reloaded, `4` stopped |
| `extension` | `str-rest` | the extension's host-stamped name |

### `core/extensions/load`, `core/extensions/unload`, `core/extensions/reload`

Runtime activation commands (ADR-024), honoured only from the sender the embedder authorised as the extension controller.

| Field | Type | Meaning |
| --- | --- | --- |
| `extension` | `str-rest` | catalog name to act on |

All three share one shape; the topic is the verb.

### `window/close-requested`

Empty payload, zero bytes.

The topic is the whole message.

## `input/*`

### `input/key-down`, `input/key-up`

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | `str-rest` | key name, e.g. `"A"`, `"Space"`, `"Left"` |

`key` is SDL's name for the key, the same on every platform and layout-independent. Letters are upper case, `A` to `Z`, and digits are `0` to `9`. The arrows are `Left`, `Right`, `Up`, and `Down`. Other common names are `Space`, `Return`, `Escape`, `Backspace`, `Tab`, `Left Shift`, `Left Ctrl`, `F1` to `F12`, and `Keypad Enter`.

### `input/mouse-down`, `input/mouse-up`

| Field | Type | Meaning |
| --- | --- | --- |
| `button` | `u8` | platform button index; 1 is left |
| `x`, `y` | `f32` each | window coordinate |

### `input/mouse-move`

| Field | Type | Meaning |
| --- | --- | --- |
| `x`, `y` | `f32` each | window coordinate |
| `dx`, `dy` | `f32` each | movement since the previous event |

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

| Field | Type | Meaning |
| --- | --- | --- |
| `r`, `g`, `b`, `a` | `u8` each | the color to clear to |

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
| `dst_x`, `dst_y` | `i32` each | destination rectangle |
| `dst_w`, `dst_h` | `u32` each | destination rectangle |
| `src_x`, `src_y` | `i32` each | source rectangle within the sprite |
| `src_w`, `src_h` | `u32` each | source rectangle within the sprite |
| `layer` | `u8` | |
| `angle` | `f32` | rotation in degrees |
| `flip_h` | `bool` | |
| `flip_v` | `bool` | |
| `tint_r`, `tint_g`, `tint_b`, `tint_a` | `u8` each | tint color, multiplied into the image |

### `gfx/draw-rect`

| Field | Type | Meaning |
| --- | --- | --- |
| `x` | `i32` | |
| `y` | `i32` | |
| `w` | `u32` | |
| `h` | `u32` | |
| `filled` | `bool` | |
| `r`, `g`, `b`, `a` | `u8` each | color |
| `layer` | `u8` | |
| `screen_space` | `bool` | |

### `gfx/draw-line`

| Field | Type | Meaning |
| --- | --- | --- |
| `x1` | `i32` | |
| `y1` | `i32` | |
| `x2` | `i32` | |
| `y2` | `i32` | |
| `r`, `g`, `b`, `a` | `u8` each | color |
| `layer` | `u8` | |

### `gfx/draw-circle`

| Field | Type | Meaning |
| --- | --- | --- |
| `x` | `i32` | |
| `y` | `i32` | |
| `radius` | `u32` | |
| `filled` | `bool` | |
| `r`, `g`, `b`, `a` | `u8` each | color |
| `layer` | `u8` | |

### `gfx/draw-triangle`

| Field | Type | Meaning |
| --- | --- | --- |
| `x1` | `i32` | |
| `y1` | `i32` | |
| `x2` | `i32` | |
| `y2` | `i32` | |
| `x3` | `i32` | |
| `y3` | `i32` | |
| `filled` | `bool` | |
| `r`, `g`, `b`, `a` | `u8` each | color |
| `layer` | `u8` | |

### `gfx/draw-text`

`text` comes **first** and is length-prefixed, because fixed fields follow it.

| Field | Type | Meaning |
| --- | --- | --- |
| `text` | `str` | |
| `x` | `i32` | |
| `y` | `i32` | |
| `size` | `u16` | point size |
| `r`, `g`, `b`, `a` | `u8` each | color |
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

Published by the renderer, except `renderer/capture`, which asks it for an image.

### `renderer/display-changed`, `renderer/logical-canvas`

| Field | Type |
| --- | --- |
| `width` | `u32` |
| `height` | `u32` |

### `renderer/capture`

Asks the renderer for an image of the next frame it presents. Published to the renderer, unlike the rest of `renderer/*`.

| Field | Type | Meaning |
| --- | --- | --- |
| `request_id` | `u32` | chosen by the caller, echoed on the reply |
| `scope` | `u8` | what to capture: `0` canvas: what the renderer drew, `gfx/*` and egui; works everywhere, an offscreen window included, `1` window: the whole window once shown, web panels included; needs a visible window on Windows or X11 |

### `renderer/captured`

The reply to `renderer/capture`: for the canvas, published once that frame is drawn and before it is shown; for the window, once it is shown.

| Field | Type | Meaning |
| --- | --- | --- |
| `request_id` | `u32` | the request this answers |
| `scope` | `u8` | what the image shows, as in `renderer/capture`; a window capture the platform cannot make is answered with the canvas |
| `width` | `u32` | image width in physical pixels; `0` when the capture failed |
| `height` | `u32` | image height in physical pixels; `0` when the capture failed |
| `png` | `rest` | the frame as a PNG file; empty when the capture failed |

## `ui/*`

### `ui/spec`

A whole widget panel, republished whenever it changes (ADR-005).

| Field | Type | Meaning |
| --- | --- | --- |
| `title` | `str` | |
| `count` | `u16` | number of widgets that follow |
| `widgets` | `count` × widget | back to back, no separator |

#### widget

Each widget is a tagged union.

| Tag | Variant | Fields after the tag |
| ---: | --- | --- |
| 0 | label | `text`: `str` |
| 1 | text-edit | `id`: `u32`; `text`: `str` |
| 2 | button | `id`: `u32`; `label`: `str` |

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

| Field | Type | Meaning |
| --- | --- | --- |
| `volume` | `f32` | `0.0` to `1.0` |

### `audio/stop-music`

Empty payload, zero bytes.

## `game-core/*`

### `game-core/entity-op`

One tagged union, the whole payload (ADR-023 — the operation set grows additively, so a decoder must expect tags it does not know).

| Tag | Variant | Fields after the tag |
| ---: | --- | --- |
| 0 | spawn | spawn (below) |
| 1 | set-velocity | `entity_id`: `u32`; `vx`, `vy`: `f32` |
| 2 | despawn | `entity_id`: `u32` |
| 3 | set-color | `entity_id`: `u32`; `r`, `g`, `b`, `a`: `u8` |
| 4 | set-debug-hitboxes | `enabled`: `bool` |
| 5 | set-paused | `paused`: `bool` |
| 6 | set-camera-follow | `entity_id`: `u32`; `viewport_w`, `viewport_h`, `zoom`: `f32` |
| 7 | set-sprite | `entity_id`: `u32`; sprite presentation (below) |
| 8 | set-camera-smoothing | `responsiveness`: `f32` |
| 9 | reset | none |
| 10 | set-sprite-tint | `entity_id`: `u32`; `r`, `g`, `b`, `a`: `u8` |

#### spawn

The fields of tag 0.

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

#### sprite presentation

The fields of tag 7 after `entity_id`.

| Field | Type | Meaning |
| --- | --- | --- |
| `sprite_id` | `u32` | |
| `frame_w` | `u32` | |
| `frame_h` | `u32` | |
| `frame_count` | `u32` | |
| `frame_duration` | `f32` | seconds per frame |
| `frames_per_row` | `u32` | |
| `draw_w` | `u32` | |
| `draw_h` | `u32` | |
| `looping` | `bool` | |
| `advance_while_stopped` | `bool` | |
| `flip_h` | `bool` | |
| `flip_v` | `bool` | |

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

#### image

One tileset image.

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
| `owner` | `str` | the extension that owns the panel |
| `panel` | `str` | owner-local panel id |
| `reason` | `str-rest` | human-readable failure |

### `web/page-message`

| Field | Type | Meaning |
| --- | --- | --- |
| `owner` | `str` | the extension that owns the panel |
| `panel` | `str` | owner-local panel id |
| `json` | `str-rest` | the page's message, as JSON text |

The JSON is not parsed by the engine. It is carried verbatim between the page and the owning extension.

## `instance/*`

Published by the instance module when a later launch of a single-instance application hands its arguments to the running one (ADR-035). The application's own startup arguments are never published here.

### `instance/launched`

| Field | Type | Meaning |
| --- | --- | --- |
| `cwd` | `str` | the launching process's working directory |
| `count` | `u16` | number of arguments that follow |
| `args` | `count` × `str` | the arguments, without the program name, in order |

## `desktop/*`

Request and result for registering one application with the desktop (ADR-036). Which application, and under which names, is fixed by whoever composed the engine; a request only chooses the action. Replies arrive in the order the work finishes, so `request_id` pairs each result with its request.

### `desktop/request`

| Field | Type | Meaning |
| --- | --- | --- |
| `request_id` | `u32` | chosen by the caller, echoed on the result |
| `action` | `u8` | `0` register, `1` unregister, `2` status |

### `desktop/result`

| Field | Type | Meaning |
| --- | --- | --- |
| `request_id` | `u32` | the request this answers |
| `tag` | `u8` | which variant follows, below |

| Tag | Variant | Fields after the tag |
| ---: | --- | --- |
| 0 | registered | `registered`: `bool` — whether every part of the registration is now present |
| 1 | failed | `reason`: `str-rest` |

## `os/*`

Desktop OS capabilities. Every completion is broadcast on os/reply with host-stamped requester and caller-chosen request_id; match both. The same completion is also sent on legacy os/result, whose IDs need application-wide coordination. Replies may arrive out of order. At most 16 requests run at once, with no queue; excess requests receive an actionable busy error. Shutdown suppresses late replies without waiting for native operations.

### `os/request`

| Field | Type | Meaning |
| --- | --- | --- |
| `request_id` | `u32` | chosen by the caller, echoed on the result |
| `action` | `u8` | `0` read clipboard, `1` write clipboard, `2` open URL, `3` pick file, `4` pick folder, `5` open directory, `6` fetch URL |
| `value` | `str` | the action's argument: text to copy, a URL, a dialog title, or a directory; empty when it takes none |

### `os/result`

| Field | Type | Meaning |
| --- | --- | --- |
| `request_id` | `u32` | the request this answers |
| `accepted` | `bool` | whether the action produced an answer; a cancelled dialog is `false` with an empty `error` |
| `value` | `str` | the answer: clipboard text, a chosen path, or for a fetch `{content-type};base64,{data}` |
| `error` | `str` | why the action failed; empty unless it did |

### `os/reply`

| Field | Type | Meaning |
| --- | --- | --- |
| `requester` | `str` | host-stamped sender of the request; match this and request_id |
| `request_id` | `u32` | the request this answers |
| `accepted` | `bool` | whether the action produced an answer; a cancelled dialog is `false` with an empty `error` |
| `value` | `str` | the answer: clipboard text, a chosen path, or for a fetch `{content-type};base64,{data}` |
| `error` | `str` | why the action failed; empty unless it did |

## `upgrader/*`

Request and result for self-update (ADR-037). Where releases come from is fixed by whoever composed the engine; a request only chooses the action. Replies arrive in the order the work finishes, so `request_id` pairs each result with its request.

### `upgrader/request`

| Field | Type | Meaning |
| --- | --- | --- |
| `request_id` | `u32` | chosen by the caller, echoed on the result |
| `action` | `u8` | `0` check, `1` stage, `2` install, `3` last-applied |

- **check** reads the release manifest and reports whether it is newer than the running version.
- **stage** downloads and verifies a newer release into its own version folder; the launcher starts it next time.
- **install** copies the running build into the install location — the "install this portable copy" action.
- **last-applied** reports the version that ran before this one, so an application can say it was just updated.

### `upgrader/result`

| Field | Type | Meaning |
| --- | --- | --- |
| `request_id` | `u32` | the request this answers |
| `tag` | `u8` | which variant follows, below |

| Tag | Variant | Fields after the tag |
| ---: | --- | --- |
| 0 | available | `version`: `str-rest` — a newer release exists |
| 1 | up-to-date | none |
| 2 | staged | `version`: `str-rest` — staged for the next start |
| 3 | installed | `fresh`: `bool` — true when nothing was installed before, so the files went straight to the install location |
| 4 | last-applied | `has_previous`: `bool`; `previous`: `str`; `current`: `str-rest` |
| 5 | failed | `reason`: `str-rest` |

`has_previous` is false on the first run of an installed application and on every run outside an install; `previous` is then empty. An application shows "updated" only when `has_previous` is true and `previous` differs from `current`.

## Direct-send endpoints

A direct `send` (ADR-010) names an endpoint rather than a topic, and completes within the call with a reply. Three endpoints are core-owned.

### `persistence`, `persistence/save`

| Field | Type | Meaning |
| --- | --- | --- |
| `bytes` | `rest` | the whole state to keep, opaque to the engine |

- **Save** — publish to the topic `persistence/save` with the payload above. The sender's own name selects the slot; an extension cannot write another's.
- **Load** — `send` to `persistence` with an empty payload. The reply is the previously saved bytes, or empty if there is nothing saved.

An empty reply is not distinguishable from an empty saved state. That is deliberate: the caller's next move is the same either way.

### `files`

`send` to `files` with a path.

| Field | Type | Meaning |
| --- | --- | --- |
| `path` | `str-rest` | relative to a directory the embedder granted |

The reply is the file's raw bytes, or empty.

Empty covers every failure — no such file, not a file, over the size limit, or a path that resolved outside the granted root — for the same reason as `persistence`. The extension is never told the root and cannot escape it.

### `web`

`send` to `web` with a tagged-union payload. Panels report back on `web/*`, above.

| Tag | Variant | Fields after the tag |
| ---: | --- | --- |
| 0 | open | `panel`: `str`; `source_kind`: `u8` — `0` inline HTML, `1` URL; `source`: `str-rest` — the HTML text or the URL, as `source_kind` says |
| 1 | close | `panel`: `str-rest` |
| 2 | navigate | `panel`: `str`; `url`: `str-rest` |
| 3 | send-json | `panel`: `str`; `json`: `str-rest` |

## Conformance

[vectors/](vectors/README.md) holds machine-readable test vectors: for each topic, a payload as hex and the field values it decodes to. An implementation is conformant when it decodes every vector to the stated values and re-encodes those values to the identical bytes.

The vectors are generated from `bones-messages` and checked against it by its own test suite, so they cannot drift from the engine. Check against the vectors rather than against this prose — the prose is here to be understood, the vectors to be executed.
