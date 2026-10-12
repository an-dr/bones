# desktop

Registers an installed application with the desktop it runs on, and removes that registration again. Registration is what makes an application more than a folder of executables: an entry in the system's application list, and a way to open a folder in it from the file manager. Nothing here copies, moves, or deletes the application's own files; an unregistered application is simply portable.

The crate offers registration two ways ([ADR-036](../../../docs/adr/ADR-036-desktop-registration-is-its-own-crate-and-opt-in-module.md)):

- **Directly.** `register`, `unregister`, and `is_registered` are plain functions. An application's `--install` calls them after the upgrade module has placed the files, with no kernel running.
- **Over the bus.** `DesktopModule` offers the same three operations to a page or guest, for one application the composition fixes.

Off by default. Enable the kernel's `desktop` feature: the functions are in `bones_kernel::desktop`, and `.desktop(integration)` adds the module.

## What a registration is

An `Integration` describes the application:

| Field | Meaning |
| --- | --- |
| `id` | Stable machine name, used for file and key names (`commits`) |
| `name` | Name shown in the application list (`Commits`) |
| `comment` | One-line description shown beside the name |
| `open_label` | Label of the folder action (`Open in Commits`) |
| `categories` | freedesktop.org menu categories, each followed by `;` (`Development;RevisionControl;`); Linux only |
| `executable` | Absolute path of the program to start; a folder is passed as its only argument |
| `icon_png` | PNG bytes of the application icon |

`register` writes every part, `unregister` removes every part, and `is_registered` reports whether every part is present. Both writes are idempotent: registering twice leaves one registration, and unregistering something that is not registered succeeds. A registration with only some of its parts present counts as not registered, so registering again is how it is repaired.

Every location is resolved through `Locations`, which a caller normally takes from `Locations::for_current_user()`. Tests point it at a scratch directory and, on Windows, a scratch registry key instead, so nothing reaches the real desktop.

Register the installed launcher, not a version folder's executable: the launcher is the one path that survives updates.

## Linux

Registration follows the freedesktop.org specifications, per user and with no elevated rights:

| Part | Location |
| --- | --- |
| Desktop entry | `$XDG_DATA_HOME/applications/<id>.desktop` |
| Icon | `$XDG_DATA_HOME/icons/hicolor/256x256/apps/<id>.png` |

`$XDG_DATA_HOME` defaults to `~/.local/share`. The desktop entry makes the application appear in the application list, and its `MimeType=inode/directory;` makes it an "Open With" choice for folders in file managers that read desktop entries, including GNOME Files (Ubuntu, Pop!_OS 22.04) and COSMIC Files (Pop!_OS 24.04). `Exec` passes the chosen folder as `%f`, and starting the application from the list passes nothing.

After writing or removing the entry, the crate runs `update-desktop-database` on the applications directory, and `gtk-update-icon-cache` on the icon theme when that theme already has a cache, so file managers see the change without a new login. A cache is never created where there was none, since it would hide icons that other applications install later without refreshing it. Both tools are best effort: a desktop without them still reads the files directly, so a missing tool is not an error.

## Windows

Registration is per user, under `HKEY_CURRENT_USER`, and needs no administrator rights:

| Part | Location |
| --- | --- |
| Folder action | `Software\Classes\Directory\shell\<id>` |
| Folder background action | `Software\Classes\Directory\Background\shell\<id>` |
| Start Menu shortcut | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\<name>.lnk` |
| Desktop shortcut | `<Desktop>\<name>.lnk` |

Each action key's default value is `open_label`, its `Icon` value is the executable, and its `command` subkey runs `"<executable>" "%1"` for a folder or `"<executable>" "%V"` for the folder whose background was clicked. These are classic shell verbs: Windows 10 shows them directly, and Windows 11 shows them under "Show more options". A top-level Windows 11 entry needs a packaged application with a COM handler, which this crate does not provide.

Both shortcuts start the executable in its own directory and take their icon from it. `is_registered` compares both actions' values with what `register` would write, and checks that both shortcuts exist.

## macOS

Not supported: `register` and `unregister` return an error, and `is_registered` reports `false`.

## Over the bus

- **One application, fixed at composition.** The `Integration` — names, icon, executable — is given to the module when the kernel is built. A guest chooses only the action: register, unregister, or status. Registration writes a command the shell runs for the user, so a guest must never choose what is registered.
- **`desktop/request` in, `desktop/result` out**, correlated by a caller-chosen id ([wit/wire-format.md](../../../wit/wire-format.md)). Every result reports whether the registration is complete after the action, so a status request and a successful write answer the same way.
- **Off the frame thread.** Each request runs on a thread of its own, so results arrive in the order the work finishes.
- **Register refuses a missing executable.** A registration pointing at nothing would start nothing; the result says where the application was expected.

When the current user's locations cannot be resolved, every request answers with that failure rather than going unanswered.
