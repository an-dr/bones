# audio

Plays sound effects and music via `audio/*` commands, backed by `kira`
(engine-agnostic, no window/`App` coupling — see ADR-019's crate-sourcing
rationale).

- `audio/load-sound` — caches decoded audio bytes (any format `kira`'s
  decoder supports) keyed by an application-assigned `u32` id, the same
  pattern as `gfx::LoadSprite`.
- `audio/play-sound` — plays a cached sound once, fire-and-forget; keeps
  playing after its handle is dropped (playback lives on the audio
  thread, not tied to the handle).
- `audio/play-music` / `audio/stop-music` / `audio/set-music-volume` — at
  most one active music track at a time (a tactical simplification, see
  `bones_messages::audio::PlayMusic`'s own doc comment).

Every `audio/*` volume field is linear amplitude (`0.0` silent, `1.0`
unity gain), converted internally to `kira`'s decibel scale — never pass
decibels over the wire.

No logger: unlike `renderer`, this can be registered via the generic
`.module(...)` path, which has no access to `Engine`'s internal logger
(see the `TODO` on `Engine::module`). Decode failures, an unknown sound
id, or the backend not yet being open are silently no-ops rather than
reported nowhere useful — the same stance `core/platform` already takes
for a gamepad that fails to open.
