# Roadmap

Add future work here only when it has a demonstrable completion artifact; completed work belongs in git history.

## Continuous integration and supply-chain checks

There is no tracked CI. `test.ps1` runs formatting, clippy with warnings denied, both feature sets, and the documentation build, which is why the gates exist at all — but it only runs where someone runs it, and today that is one Windows ARM64 machine. Everything the README calls best-effort is best-effort for exactly this reason.

Three things are missing, and they are separable. **CI** running the existing gates on a declared platform matrix, which needs the Linux and macOS questions answered first — the wry web-panel tests are gated to Windows and have never run elsewhere. **A dependency advisory and licence scan**, which matters more than usual here because the engine links its whole tree statically and ships generated third-party notices. **An MSRV policy**, or an explicit decision that the policy is current-stable-only; the README says the latter today, which is a statement, not a check.

Completion artifact: a CI configuration running the `test.ps1` gates plus an advisory scan on every platform the README claims support for, and a `rust-version` in `[workspace.package]` or a documented policy that CI enforces.

## Reproducible release archives

`dist.ps1` writes a SHA256 beside each archive, which verifies a download but not a build: zip entries carry modification times, so two runs over identical inputs produce different hashes. A consumer cannot check that the archive they downloaded is the one this repository's source produces.

Completion artifact: two runs of the release job over the same commit producing byte-identical archives, and the checksum published from source rather than from the upload.

## API documentation on the ABI-line crates

`bones-engine` enables `missing_docs`, so the embedder-facing surface cannot grow an undocumented public item. `bones-messages` and `bones-wasm-sdk` do not, and between them have roughly two hundred undocumented public items — mostly message fields and generated bindings.

The message fields are not undocumented in substance: [wit/wire-format.md](../wit/wire-format.md) specifies every one of them, for every language rather than only Rust, which is where an extension author should be reading. What is missing is the rustdoc a Rust author sees at the call site, and the lint that would keep it honest.

Completion artifact: `missing_docs` enabled on both crates with the gap closed, and the message documentation generated from or checked against the wire-format specification rather than written twice.
