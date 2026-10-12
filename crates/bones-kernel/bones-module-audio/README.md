# audio

Plays sound effects and music via `audio/*` commands, backed by `kira` ([ADR-018](../../../docs/adr/ADR-018-core-2d-presentation-input-and-persistence-gaps.md)).

- Behaviour — the effect/music split, the volume convention, and why failures are silent: [docs/design/audio.md](../../../docs/design/audio.md).
- Message field layouts: [wit/wire-format.md](../../../wit/wire-format.md).

Compiled in by default, registered only when the composition asks for it: not every deployment target has a working audio device. Implementation notes are in the source doc comments — the linear-amplitude to decibel conversion has its own module.
