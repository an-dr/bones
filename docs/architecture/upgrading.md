# Upgrading a shipped application

An application that ships outside a store has to replace itself. The problem is not downloading a new build — it is that **no running executable can overwrite itself**, so something else must do it. bones answers this with two executables that replace each other, and an install layout that makes a failed upgrade recoverable ([ADR-033](../adr/ADR-033-self-update-is-a-crate-and-its-binaries-avoid-installer-words.md)).

This is a capability an application may adopt, not part of the engine. It runs beside a bones application rather than inside the engine, touches no bus and no module, and an application that never upgrades itself omits it entirely.

## The two halves

```text
<install dir>/
├── myapp.exe        ← the launcher: permanent, the name users type
├── 1.3.0/           ← a version folder: the app and everything it needs
│   └── myapp-app.exe
└── 1.2.0/           ← the previous version, kept for rollback
```

**The launcher** is what a shortcut points at and what the shell waits on. It picks the newest version folder, starts the app there, and exits. It is deliberately thin — it interprets nothing, supervises nothing, and reads no configuration — because it is the one file the app cannot overwrite while it runs. Logic that accumulates in the launcher is logic that can never be fixed in the field.

**The app** is the real application, living in a folder named for its version. It does the upgrading: fetch a manifest, compare versions, verify a download, unpack it into a new version folder beside the current one.

Each half replaces the other. The app installs a new version folder, and the launcher starts using it on the next run. The app also refreshes the launcher itself by renaming the running file aside and writing the new one into the freed name — which is why a launcher bug is fixable at all, and why both halves belong to one mechanism rather than two.

## Why version folders

The alternative — one directory overwritten in place — makes an interrupted upgrade fatal: the moment a file is half-replaced there is no working application on disk. Version folders make an upgrade **additive**. The new version is staged completely beside the old one, and only the launcher's choice of folder makes it current. Nothing is destroyed to install something.

Three properties follow, and each of them is the reason for the layout rather than a bonus:

- **A failed download changes nothing.** The running version is untouched until a complete, checksum-verified version folder exists.
- **Rollback is a deletion.** Removing the current folder makes the previous one current again; there is no uninstall step and no backup to restore.
- **Two builds claiming the same version can coexist**, disambiguated by a suffix, which is what makes a development build that never bumps its version usable against the same mechanism.

## What the application supplies

The mechanism knows nothing about the application it updates. Four names — where it installs, the environment variable that overrides that, and what the two executables are called — are fixed **at compile time**, so the launcher and the app agree without either reading a configuration file.

That is deliberate and it solves a bootstrapping problem: a launcher that had to read a config file to learn where things live would first have to find that file, which is the very question the identity answers. Building both halves together makes disagreement impossible.

The one capability the mechanism asks of its host is the ability to fetch a URL. Everything else is filesystem and version arithmetic — which is why it works in an application with no browser, and why a test can satisfy it with canned responses instead of a network.

## A naming constraint worth knowing

Windows treats any executable whose filename contains `update`, `install`, `setup`, or `patch` as an installer and forces an elevation prompt — with no manifest asking for it, and for test binaries as readily as shipped ones. This is why the mechanism is called **upgrader**, and the same applies to a launcher an application names for itself. It is recorded in [ADR-033](../adr/ADR-033-self-update-is-a-crate-and-its-binaries-avoid-installer-words.md) because it was found by measurement and costs a day to rediscover.

## How it relates to the version lines

Upgrading moves the **engine line** — an application ships a build of the engine and its modules ([ADR-029](../adr/ADR-029-the-two-version-lines-are-the-two-public-surfaces.md)). It has nothing to say about the ABI line, and this is the case worth being careful about: extensions are not part of a version folder unless the application puts them there. An upgrade that moves the ABI leaves previously installed extensions built against the old contract, and those are refused at load rather than silently mismatched. An application that ships third-party extensions owns that compatibility question; the mechanism does not answer it.

Interface detail — the manifest shape, the install and rollback entry points, what the launcher forwards — is in the [bones-upgrader README](../../crates/bones-upgrader/README.md).
