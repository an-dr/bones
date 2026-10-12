# bones-versioning

Pure version semantics shared by the launcher and upgrade module. The third-party `semver` crate supplies parsing, validation, prereleases, build metadata, precedence, and Cargo-style version requirements. Its types are re-exported here. This crate performs no filesystem, network, installation, or process operations.

Versions require `major.minor.patch`; leading zeros, malformed identifiers, and core numbers above `u64::MAX` are rejected. Update and folder ordering use SemVer precedence, which ignores build metadata. `VersionReq` follows Cargo's requirement syntax and prerelease opt-in rules.

Installation instances use `version~instance` folder names. The separator is outside SemVer, so `1.2.3-rc.1+build.7~abcdef` preserves the complete version. Equal precedence uses supplied modification time, then path, to order repeated builds. Legacy `version-hash` names are interpreted as SemVer prereleases; an ambiguous legacy suffix cannot be automatically distinguished from an intentional prerelease.

Run `cargo test -p bones-versioning` from the repository root. See [ADR-039](../../docs/adr/ADR-039-versioning-is-pure-the-launcher-launches-and-the-upgrade-module-owns-installation.md) for the boundary.
