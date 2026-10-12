# ADR-039: Versioning is pure, the launcher launches, and the upgrade module owns installation

## Problem

The self-update crate combines launching, installation, downloads, application identity, and version arithmetic. Moving those responsibilities into one module would still leave a standalone upgrading library behind a different name. The application and launcher need common version semantics, not a common update engine.

## Decision

- `bones-versioning` contains only version parsing, comparison, folder-name interpretation, and ordering of supplied version metadata. It performs no filesystem operations and knows no application identity.
- The third-party `semver` crate owns SemVer parsing and validation; its types and Cargo-style requirements are re-exported by versioning. Selection and update checks compare precedence, ignoring build metadata.
- Collision folders use `version~instance`, preserving prerelease and build identifiers. Legacy hyphen suffixes are interpreted as prereleases because their original intent cannot be inferred safely.
- `bones-launcher` discovers installed versions, selects an executable, launches it, and forwards application arguments. It does not install, delete versions, download, or replace itself.
- `bones-module-upgrade` owns release checks, transport, verification, installation, retention, rollback mutations, companion installation, and launcher refresh. Its implementation helpers are private; trusted host operations are methods of the module, sharing the same implementation as bus requests.
- Application composition supplies identity and paths. The launcher receives only the executable name and installation directory it needs to launch.
- A launcher can select the previous version for recovery without deleting anything. Application arguments, including installation and rollback commands, belong to the application.
- The existing `upgrader/*` wire contract remains unchanged. Rust crate and composition names become `upgrade`.
- This supersedes the combined crate boundary in ADR-033 and the standalone update functions in ADR-037. Their release-source trust rule and Windows executable naming constraint still apply.

## Rationale

Both executables share pure version rules without depending on each other. The launcher cannot accidentally acquire the kernel, TLS, or archive handling through its dependency graph. Update behavior has one implementation inside the native module, usable by trusted application startup code without a second standalone library.

## Rejected alternatives

- An optional bus feature on a standalone updater preserves the abstraction being removed.
- Installation identity or filesystem helpers inside versioning turn it into an installation library.
- Rollback deletion inside the launcher makes it an installer as well as a launcher.
