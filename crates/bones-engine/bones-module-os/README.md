# bones-module-os

Desktop capabilities for sandboxed guests: clipboard, browser, native file dialogs and HTTPS fetch.

An extension has none of these, because the sandbox that makes it safe to load also cuts it off from the machine. This module is the trusted side of that split. It subscribes to `os/request`, does the work, and answers on `os/result`; the message types are in `bones-messages::os`.

Each request runs on its own thread. A file dialog blocks until the user chooses and a fetch until the remote answers, neither of which may stall the engine, so replies arrive when the work finishes rather than in the order asked. The caller-chosen request id is what pairs an answer with its question.

`OsBackend` is the seam: the module owns the bus protocol and nothing else, so a host can supply a backend that opens no dialogs, or one for a platform this crate does not cover, without reimplementing the plumbing.

Enable it with the engine's `os` feature, then `.os()` on the builder. It is off by default because it pulls in a TLS stack and native dialogs that a headless or game-only build has no use for.
