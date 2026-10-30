# Desktop capabilities

Detailed design of the `os` module: the capabilities a sandboxed guest cannot have, offered over the bus instead. Its place among the modules is in [architecture/overview.md](../architecture/overview.md).

Decision: [ADR-034](../adr/ADR-034-os-capabilities-are-an-optional-module-splittable-per-capability.md) — an optional module, off by default, expected to split per capability as it grows.

## The problem it solves

The sandbox that makes an extension safe to load also cuts it off from the machine ([ADR-001](../adr/ADR-001-wasm-component-model.md)). An extension cannot read the clipboard, open a browser, show a file picker, or reach the network — and for most of what an extension does, that is exactly right.

But a desktop application needs those things, and the extension is where the application's behaviour lives. This module is the trusted side of that split: it holds the capability, the extension asks for it, and the sandbox stays intact. The same pattern persistence uses ([ADR-018](../adr/ADR-018-core-2d-presentation-input-and-persistence-gaps.md)) — a trusted native module wrapping an OS-facing concern, reached over the bus.

Four capabilities: clipboard access, opening a URL in the user's browser, native file and folder dialogs, and HTTPS fetch.

## Request and answer

Requests use `os/request`. Each completion is published on `os/reply` with the **host-stamped requester** and **caller-chosen request id**. Callers subscribe to `os/reply`, filter by their extension name, and match their own outstanding id. IDs must be unique among that caller's outstanding requests; different callers may reuse an id. The reply remains a broadcast, not private delivery.

For compatibility the same completion is also published on `os/result` with its original payload. That legacy topic has no requester field and requires application-wide ID coordination. New callers use `os/reply` and ignore `os/result`; the request payload is unchanged. See [ADR-045](../adr/ADR-045-os-replies-identify-the-caller-and-outstanding-work-is-bounded.md).

**Answers arrive out of order, by design.** A file dialog blocks until the user chooses; a fetch blocks until the remote answers. Neither may stall the frame, so each request runs off the frame thread and its answer is published when the work finishes. A caller that issues three requests may see the third answered first. A caller that assumes ordering will break, and nothing in the protocol hides that from it.

This is the same reasoning behind the bus's per-sender ordering guarantee ([ADR-009](../adr/ADR-009-delivery-semantics.md)) stopping where it does: ordering is promised where it is cheap and honest, not where it would be a fiction.

## Outstanding work and shutdown

The module admits at most 16 active requests across all callers, with no waiting queue. A request above the limit receives an immediate error naming the requester, the limit, and the remedy: wait for an outstanding request to complete before retrying. Worker creation failures and backend panics also produce errors and release capacity. These outcomes use both reply topics.

Shutdown closes admission and suppresses replies from work that finishes afterwards. It does not wait for or cancel an already-running backend operation: a native dialog may remain blocked on the user, and a fetch retains its normal timeout. Dropping the module also closes admission. There is no completion guarantee after shutdown begins.

## The backend seam

The module owns the bus protocol and nothing else. What actually opens a dialog or performs a fetch sits behind an interface, which is what makes three otherwise-awkward situations ordinary:

- a host that must not open dialogs at all — a kiosk, a service, a test — supplies a backend that refuses them, without the protocol changing;
- a platform this crate does not cover is supported by supplying a backend, not by reimplementing the plumbing;
- tests exercise the protocol against canned answers instead of a real desktop.

## Why it is off by default

The module is not compiled unless asked for. It pulls in a TLS stack and native dialog libraries, which a headless build or a game has no use for — and an engine that cannot open a file picker is a smaller attack surface as well as a smaller binary. Enabling it is a two-part decision, the same as any optional module: compile it in, then compose it ([architecture/overview.md](../architecture/overview.md)).

Granting these capabilities to guests is a trust decision the embedder makes, not a default the engine assumes ([ADR-034](../adr/ADR-034-os-capabilities-are-an-optional-module-splittable-per-capability.md)).

The four capabilities are grouped because each needs the host, not because they belong together — and a host wanting only a file picker currently accepts a TLS stack with it. Splitting them into per-capability modules is the anticipated move as the set grows, and costs no guest-visible change: guests already address capabilities individually over the bus.

## Where the detail lives

Message field layouts are in [wit/wire-format.md](../../wit/wire-format.md). Implementation notes belong beside the code in [bones-module-os](../../crates/bones-kernel/bones-module-os/README.md).
