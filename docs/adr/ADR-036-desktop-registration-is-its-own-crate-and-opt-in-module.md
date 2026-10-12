# ADR-036: Desktop registration is its own crate and opt-in module

## Problem

An installed application should appear in the system's application list and offer "Open in <App>" on folders in the file manager, and should be able to remove both again. Commits implemented this in its own crate; it names no repository, and a second application now needs it.

Two existing homes were close at hand. `bones-upgrader` already installs an application, so registration could be another step of an install. `bones-module-os` already performs desktop actions on a guest's behalf, so registration could be another `os/*` action. Both would have been convenient, and both would have been wrong for a reason worth recording.

## Decision

Desktop registration is **its own opt-in module crate, `bones-module-desktop`**, separate from both.

- The crate's plain functions register, unregister, and report the status of one `Integration` — names, an executable, and an icon — through per-user locations only. An application's `--install` calls them directly, with no kernel running.
- The crate's native module answers `desktop/request` with `desktop/result` for **one `Integration` fixed by the composition root**. A guest may ask to register, unregister, or read the status of that integration, and nothing else.
- `bones-upgrader` does not depend on it. An application's `--install` calls both, one after the other.

This is the first per-capability split that [ADR-034](ADR-034-os-capabilities-are-an-optional-module-splittable-per-capability.md) anticipated.

## Rationale

**Replacing versions and registering with the desktop are different responsibilities.** An application can be installed and never registered (portable use), registered and later unregistered without being uninstalled, and updated a hundred times without its registration changing. Folding one into the other would make each harder to use alone.

**A guest must not choose what is registered.** Registration writes a command line the shell will run on the user's behalf. If the executable path or the labels arrived over the bus, an extension could register any program as an "Open in" verb. Fixing the `Integration` at composition keeps that decision with whoever built the binary — the same trust rule ADR-034 applies to every OS capability — and is why this is not a set of new `os/*` actions, which carry a guest-supplied value by design.

**A separate module costs nothing to an application that does not want it.** It is compiled only behind the kernel's `desktop` feature and registered only by `.desktop(...)`.

## Consequences

- A registration written by Commits before the move reads as registered afterwards: the crate keeps the same files, registry keys, and labels for the same inputs.
- macOS has no registration; the crate reports it as unsupported rather than pretending.
