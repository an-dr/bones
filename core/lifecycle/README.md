# lifecycle

`core/lifecycle` (messaging.md, design/extensions.md): the topic every
extension state transition is published on — Loaded, Faulted, Reloading,
Reloaded, Stopped — so tooling and other extensions can observe loads,
faults, and reloads.

- `Event` — one of the five transitions.
- `publish(bus, sender, name, event)` — publishes `name`'s transition,
  `sender` stamped as the publishing component (e.g. `"engine"`).

The shared `bones-messages::lifecycle::LifecycleEvent` type owns the topic
and wire contract. Consumers decode it through the common `DecodeMessage`
interface.
