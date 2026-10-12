# bones-module-upgrade

Owns application-side upgrading: release checks, HTTPS transport, archive verification and extraction, installation, retention, rollback, companion installation, and launcher refresh. There is no standalone updater crate or public namespace of update functions.

Compose `UpgradeModule` with `UpgradeConfig`: the application supplies its `AppIdentity`, release source, current version, and optionally a `Fetch` implementation. Enable the kernel's `upgrade` feature and call `.upgrade(config)` on the builder. Trusted startup code can call the module's installation, companion-installation, rollback, and launcher-refresh methods before a bus is attached; all use the module's private implementation.

## Bus contract

The existing `upgrader/request` and `upgrader/result` wire topics are preserved. Check, stage, and install run off the frame thread, and each result repeats its request ID. The source is fixed at composition; a guest chooses an action, never a release URL or installation identity.

Last-applied reads the previously saved version on the first frame and records the current version. A first run or a run outside an installed version folder reports no previous version.

## Installation handoff

Downloads are checksum-verified when a manifest supplies a checksum. Extraction uses an unselectable scratch directory; only a completed directory is renamed to a version-folder name. The launcher discovers the new version on its next run. The previous version remains available for recovery.

Installation mutations are serialized across module instances within the process. Retention keeps the newest two versions and preserves a running version if it is older. A process interruption can leave a scratch directory, which the launcher ignores.

HTTPS transport accepts `https://` URLs, treats 404 as no response, caps downloads at 256 MB, and uses a 15-second connection timeout and a 60-second timeout per read. A host can inject a transport for a file share or tests.

## Release payload

`ReleaseSource::GitHub { owner, repo, app }` reads the latest release's `<app>-<platform>.json`. `ReleaseSource::Url` uses a host-configured manifest instead. The manifest contains `version`, the archive `url`, and an optional hexadecimal `sha256`.

Versions are strict SemVer. Prereleases participate in precedence; build metadata alone does not make an update newer. Repeated installations use `version~instance` folder names. Legacy hyphen collision suffixes are interpreted as prereleases.

The ZIP contains a version folder's contents at its root, including the application executable and, when supplied, the replacement launcher as `<launcher-stem>-launcher` with the platform executable extension. Installation gives that payload the launcher's permanent name.

## Companion applications

`UpgradeModule::install_companion` accepts a trusted companion identity and its archive. `CompanionPolicy::IfAbsent` preserves any existing installation; `IfNewer` adds a version only when the supplied build is newer. Desktop registration remains the companion application's responsibility.

Run `cargo test -p bones-module-upgrade` from the repository root. See [ADR-039](../../../docs/adr/ADR-039-versioning-is-pure-the-launcher-launches-and-the-upgrade-module-owns-installation.md) for the boundary.
