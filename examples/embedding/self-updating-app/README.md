# self-updating-app

An application that replaces itself with a newer copy, and the launcher that starts whichever copy is current.

This is the shape every desktop application shipping outside a store ends up needing, and the upgrade module is the part of it that is the same for everyone: comparing versions, unpacking into version folders, replacing an entry point, rolling back. What an application supplies is its identity, a way to fetch bytes, and the decision about when to update.

## Why two binaries

An application cannot overwrite its own running executable. So the install has two parts:

- `demo` — the launcher, at the install root. It never changes, it is the name users type, and its only job is to find the newest version folder and start the app inside it. `bones_launcher::run` is the entire implementation.
- `demo-app` — the application, inside a version folder like `1.0.0/`. This is what gets replaced.

An update never touches the running process. It unpacks a new version folder beside the current one, and the launcher picks it up on the next start. A failed update costs the update, not the installation.

## What the example does

`cargo xtask example self-updating-app` builds the app twice, at 1.0.0 and 1.1.0. It installs 1.0.0, and publishes 1.1.0 as a zip with a manifest pointing at it — so there is something real to update to, which is the part a single-version example cannot show.

```sh
cargo xtask example self-updating-app
```

Then, with the install directory the command prints:

```sh
$env:DEMO_APP_INSTALL_DIR = '.../dist/app'   # PowerShell
.../dist/app/demo                            # first run: reports 1.0.0, stages 1.1.0
.../dist/app/demo --version                  # now reports 1.1.0
.../dist/app/demo --previous --rollback                 # back to 1.0.0
```

The update runs through the kernel's `upgrade` module, the way a page or an extension would ask for one: the app builds a headless kernel with `.upgrade(config)`, publishes `upgrader/request` with the stage action, and waits for the `upgrader/result` with the same id. The module checks the manifest, verifies the download, and unpacks it into a version folder ([ADR-037](../../../docs/adr/ADR-037-the-update-flow-is-functions-and-its-bus-module-owns-the-download.md)).

The app fetches over `file://` rather than HTTPS, so the example needs no server: `UpgradeConfig::fetch` swaps in its own `Fetch`. `Fetch` is one method wide for exactly this reason. A real app leaves it unset and gets the module's HTTPS download.

## Installing and registering

```sh
demo-app --install      # copy this build into the install location, then register it
demo-app --unregister   # remove the registration; the installed files stay
```

`--install` is what a real application's installer step looks like: `UpgradeModule::install` places the files — a fresh install, or a new version folder beside an existing one — and then `bones_module_desktop::register` adds the installed launcher to the application list and as "Open in Bones Demo App" on folders. Two crates called one after the other, because replacing versions and appearing on the desktop are separate jobs ([ADR-036](../../../docs/adr/ADR-036-desktop-registration-is-its-own-crate-and-opt-in-module.md)). It registers on the real desktop, so run `--unregister` afterwards.

## Identity

The upgrade module spells no application into itself. The application composes its identity from four build-time names through `option_env!`, in [app/.cargo/config.toml](app/.cargo/config.toml):

| Variable | This example | What it names |
| --- | --- | --- |
| `UPGRADER_INSTALL_DIR_NAME` | `.demo-app` | the folder under the user's home |
| `UPGRADER_INSTALL_ENV_VAR` | `DEMO_APP_INSTALL_DIR` | the override, used here to install into `dist/` instead of `~` |
| `UPGRADER_LAUNCHER_STEM` | `demo` | the entry point users run |
| `UPGRADER_APP_STEM` | `demo-app` | the executable inside a version folder |

Both binaries are built with matching executable names. The launcher uses its own directory as the installation root and reads no configuration file.

The xtask build also sets these as real environment variables. Cargo merges `[env]` from every ancestor directory, so a clone sitting inside another cargo project would otherwise inherit that project's values for the same keys — which is a trap worth knowing about if your own app lives inside a larger tree.

## Things worth copying

- **The checksum is optional in the manifest format and supplied here.** `download_asset_verified` refuses a mismatch outright, treating a truncated download exactly like a network failure rather than unpacking it.
- **`Ok(None)` is not an error.** A manifest that has not been published yet is a normal answer, which is why `Fetch::fetch_url` returns an `Option`.
- **The launcher only selects and launches.** `--previous` selects the previous installed version without deleting anything. Application flags, including `--version`, `--help`, and `--rollback`, are forwarded. To recover, `demo --previous --rollback` starts the previous application and lets its upgrade module remove the newest version. The launcher detaches; run `demo-app` directly when a terminal needs to wait for command output.
- **`build.rs` declares `rerun-if-env-changed`.** `option_env!` is invisible to cargo, so without it, building 1.1.0 then 1.0.0 from one source tree silently reuses the first binary — and the example would demonstrate an update that never happened.
