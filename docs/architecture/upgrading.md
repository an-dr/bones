# Upgrading a shipped application

An application that ships outside a store has to replace itself. The problem is not downloading a new build — it is that **no running executable can overwrite itself**, so something else must do it. bones answers this with two executables that replace each other, and an install layout that makes a failed upgrade recoverable ([ADR-033](../adr/ADR-033-self-update-is-a-crate-and-its-binaries-avoid-installer-words.md)).

This is a capability an application may adopt, not part of the engine. The launcher runs beside the application; the optional upgrade module runs inside it. They share only pure version semantics. An application that never upgrades itself omits the module entirely.

## The two halves

```text
<install dir>/
├── myapp.exe        ← the launcher: permanent, the name users type
├── 1.3.0/           ← a version folder: the app and everything it needs
│   └── myapp-app.exe
└── 1.2.0/           ← the previous version, kept for rollback
```

**The launcher** is what a shortcut points at. It picks the newest version folder, starts the app there, and exits. It is deliberately thin — it interprets nothing, supervises nothing, and reads no configuration — because it is the one file the app cannot overwrite while it runs. Logic that accumulates in the launcher is logic that can never be fixed in the field.

**The app** is the real application, living in a folder named for its version. It does the upgrading: fetch a manifest, compare versions, verify a download, unpack it into a new version folder beside the current one.

Each half replaces the other. The app installs a new version folder, and the launcher starts using it on the next run. The app also refreshes the launcher itself by renaming the running file aside and writing the new one into the freed name — which is why a launcher bug is fixable at all, and why both halves belong to one mechanism rather than two.

## Why version folders

The alternative — one directory overwritten in place — makes an interrupted upgrade fatal: the moment a file is half-replaced there is no working application on disk. Version folders make an upgrade **additive**. The new version is staged completely beside the old one, and only the launcher's choice of folder makes it current. Nothing is destroyed to install something.

Three properties follow, and each of them is the reason for the layout rather than a bonus:

- **A failed download changes nothing.** The running version is untouched until a complete, checksum-verified version folder exists.
- **Rollback is an upgrade operation.** The upgrade module can remove the newest folder when a previous version exists. For a build that cannot start, the launcher can select the previous version without modifying the installation; that application can then perform recovery.
- **SemVer determines precedence.** Prereleases are ordered correctly; build metadata does not make an update newer. The shared `bones-versioning` crate delegates SemVer to the third-party `semver` library.
- **Two builds claiming the same version can coexist**, disambiguated by a `~instance` suffix distinct from SemVer, which is what makes a development build that never bumps its version usable against the same mechanism.

## What the application supplies

The mechanism knows nothing about the application it updates. The application supplies its installation identity and release source when it composes the upgrade module. Its launcher needs only the installation directory and application executable name.

The launcher uses its own directory as the installation root. The application builds both executables with matching names, so launching needs no configuration-file lookup or dependency on the installation module.

The upgrade module provides HTTPS and accepts a host-supplied transport when needed. Everything else is filesystem and version arithmetic — which is why it works in an application with no browser, and why a test can satisfy it with canned responses instead of a network.

## Where releases come from

A release is two files per platform: an archive holding one version folder's contents, and a manifest naming its version, its URL, and its checksum. The usual home is the latest release of a GitHub repository, where the manifest has a fixed name per platform, so an application finds its next version without knowing it in advance. The platform is fixed when the application is built, so an installed copy is never offered another platform's archive.

Which source an application reads — that convention, or a URL a user configured instead — is decided by the application when it starts, never by a page or an extension asking for an update ([ADR-037](../adr/ADR-037-the-update-flow-is-functions-and-its-bus-module-owns-the-download.md)). A guest that could choose the URL could make the application install any binary.

## Updating from inside a running application

The upgrade module owns the update flow and its private implementation. Trusted application startup code invokes operations through the module; bus requests invoke that same implementation off the frame thread. There is no standalone upgrading library ([ADR-039](../adr/ADR-039-versioning-is-pure-the-launcher-launches-and-the-upgrade-module-owns-installation.md)). The module owns its HTTPS download, separate from the `os` module's fetch, because a release archive is far larger than anything a guest should fetch for itself.

The module also remembers the version that ran last, so an application can tell its user it was just updated. A first run is never reported as an update, and nothing is recorded for a build running outside an install.

## Installing a companion application

An application can ship another bones application inside its own release and install it on the user's behalf — an editor that brings its git client, for example. The companion's archive is that companion's own update payload, unchanged, so it installs exactly as if the companion had installed itself.

By default an existing companion is never touched, newer or older, because a companion keeps itself up to date. An application that wants to bring an older companion forward may ask for that instead, and the companion gains a new version folder exactly as a self-update would give it one. Installing a companion never registers it with the desktop; the companion does that itself.

## A naming constraint worth knowing

Windows treats any executable whose filename contains `update`, `install`, `setup`, or `patch` as an installer and forces an elevation prompt — with no manifest asking for it, and for test binaries as readily as shipped ones. This is why the mechanism is called **upgrader**, and the same applies to a launcher an application names for itself. It is recorded in [ADR-033](../adr/ADR-033-self-update-is-a-crate-and-its-binaries-avoid-installer-words.md) because it was found by measurement and costs a day to rediscover.

## How it relates to the version lines

Upgrading moves the **engine line** — an application ships a build of the engine and its modules ([ADR-029](../adr/ADR-029-the-two-version-lines-are-the-two-public-surfaces.md)). It has nothing to say about the ABI line, and this is the case worth being careful about: extensions are not part of a version folder unless the application puts them there. An upgrade that moves the ABI leaves previously installed extensions built against the old contract, and those are refused at load rather than silently mismatched. An application that ships third-party extensions owns that compatibility question; the mechanism does not answer it.

Interfaces are described in the [upgrade module](../../crates/bones-kernel/bones-module-upgrade/README.md), [launcher](../../crates/bones-launcher/README.md), and [versioning](../../crates/bones-versioning/README.md) READMEs.
