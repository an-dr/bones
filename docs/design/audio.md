# Audio

Detailed design of the `audio` module: sound effects and music, requested over the bus. Its place among the modules is in [architecture/overview.md](../architecture/overview.md).

Decision: [ADR-018](../adr/ADR-018-core-2d-presentation-input-and-persistence-gaps.md) — audio is a native module wrapping a bought, engine-agnostic backend rather than a mixer bones owns.

## The shape of it

Audio follows the same principle as rendering ([ADR-002](../adr/ADR-002-engine-owned-rendering.md)): the extension says *what*, the engine does it. An extension never opens a device, decodes a file, or mixes a buffer — it publishes a command and the module plays.

Two kinds of sound, treated differently because they are used differently:

- **Effects** are loaded once and played many times. A load caches decoded audio under a caller-assigned id; a play names that id. Playback is fire-and-forget — it continues to its end whether or not the caller does anything further, which is what a footstep or an impact needs.
- **Music** is a single track. At most one plays at a time, and starting another replaces it. That is a deliberate simplification rather than an oversight: a caller that needs layered tracks is describing something this vocabulary does not cover, and extending it is a design change rather than a parameter.

Loading is separate from playing for the same reason sprites are: decoding on every play would put file I/O in the frame path.

## Volume is amplitude

Every volume field is **linear amplitude** — `0.0` silent, `1.0` unity gain — not decibels. The wire format says so and the module converts internally to whatever its backend wants.

This is the kind of decision worth stating once, loudly, in the place callers read: a caller passing decibels over the wire will produce something that plays, quietly and wrongly, with nothing failing to indicate why. A unit mismatch that still runs is worse than one that errors.

## Failure is quiet

An unknown sound id, a file that will not decode, a device that is not open — none of these fault the extension or stop the frame. They do nothing.

That is the same stance the platform layer takes toward a gamepad that fails to open, and it follows from where audio sits: a game that loses its sound is diminished, while a game that stops is broken. The module is optional; behaving as though it were absent is the correct failure mode for it. A caller that needs to know whether a sound played is asking for a guarantee this vocabulary does not offer.

## Optional twice over

Audio is compiled in by default but registered only when the composition asks for it, because not every deployment target has a working audio device — a CI runner and many containers do not ([architecture/overview.md](../architecture/overview.md)). A build that never registers it carries the code and pays nothing at runtime.

## Where the detail lives

Message field layouts are in [wit/wire-format.md](../../wit/wire-format.md). Implementation notes — the backend, the amplitude conversion, how caching works — belong beside the code in [bones-module-audio](../../crates/bones-engine/bones-module-audio/README.md).
