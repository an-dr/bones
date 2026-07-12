# lifecycle

`core/lifecycle` (messaging.md, design/extensions.md): the topic every
extension state transition is published on — Loaded, Faulted, Reloading,
Reloaded, Stopped — so tooling and other extensions can observe loads,
faults, and reloads.

- `Event` — one of the five transitions.
- `publish(bus, sender, name, event)` — publishes `name`'s transition,
  `sender` stamped as the publishing component (e.g. `"engine"`).
- `parse(payload)` — recovers `(event, name)` from a received envelope's
  payload.
