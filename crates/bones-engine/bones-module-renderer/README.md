# renderer

Executes `gfx/*` draw commands against one SDL window ([ADR-002](../../../docs/adr/ADR-002-engine-owned-rendering.md)). Takes the window from `platform` rather than opening its own, and decodes commands with `bones-messages`.

- Behaviour, layering, and the retained-batch model: [docs/design/presentation.md](../../../docs/design/presentation.md).
- Command field layouts: [wit/wire-format.md](../../../wit/wire-format.md), which specifies them for every language.

Implementation notes live in the source doc comments — `Renderer`'s own explains why it is wrapped in `SendWrapper` rather than being `Send` on its own.
