# os

Desktop capabilities for sandboxed guests: clipboard, browser, native file dialogs and HTTPS fetch ([ADR-034](../../../docs/adr/ADR-034-os-capabilities-are-an-optional-module-splittable-per-capability.md)). The trusted side of the sandbox boundary — an extension asks over the bus and this module does the work.

- How requests are correlated, why answers arrive out of order, and the backend seam: [docs/design/os.md](../../../docs/design/os.md).
- Message field layouts: [wit/wire-format.md](../../../wit/wire-format.md).

Off by default: it pulls in a TLS stack and native dialogs a headless or game-only build has no use for. Enable the engine's `os` feature, then `.os()` on the builder.
