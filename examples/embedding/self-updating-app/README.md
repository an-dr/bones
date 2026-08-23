# self-updating-app

An application that replaces itself with a newer copy, and the launcher that starts whichever copy is current.

This is the shape every desktop application shipping outside a store ends up needing, and `bones-upgrader` is the part of it that is the same for everyone: comparing versions, unpacking into version folders, replacing an entry point, rolling back. What an application supplies is its identity, a way to fetch bytes, and the decision about when to update.

## Why two binaries

An application cannot overwrite its own running executable. So the install has two parts:

- `demo` — the launcher, at the install root. It never changes, it is the name users type, and its only job is to find the newest version folder and start the app inside it. `launcher::run` is the entire implementation.
- `demo-app` — the application, inside a version folder like `1.0.0/`. This is what gets replaced.

An update never touches the running process. It unpacks a new version folder beside the current one, and the launcher picks it up on the next start. A failed update costs the update, not the installation.

## What the example does

`build.ps1` builds the app twice, at 1.0.0 and 1.1.0. It installs 1.0.0, and publishes 1.1.0 as a zip with a manifest pointing at it — so there is something real to update to, which is the part a single-version example cannot show.

```sh
pwsh examples/embedding/self-updating-app/build.ps1
```

Then, with the install directory the build script prints:

```sh
$env:DEMO_APP_INSTALL_DIR = '.../dist/app'   # PowerShell
.../dist/app/demo                            # first run: reports 1.0.0, stages 1.1.0
.../dist/app/demo --version                  # now reports 1.1.0
.../dist/app/demo --rollback                 # back to 1.0.0
```

The app fetches over `file://` rather than HTTPS, so the example needs no server. `Fetch` is one method wide for exactly this reason: a real host wires it to whatever it already has, and the engine's `os` module offers `fetch_url` in one line.

## Identity

`bones-upgrader` spells no application into itself. Four names arrive at compile time through `option_env!`, in [app/.cargo/config.toml](app/.cargo/config.toml):

| Variable | This example | What it names |
| --- | --- | --- |
| `UPGRADER_INSTALL_DIR_NAME` | `.demo-app` | the folder under the user's home |
| `UPGRADER_INSTALL_ENV_VAR` | `DEMO_APP_INSTALL_DIR` | the override, used here to install into `dist/` instead of `~` |
| `UPGRADER_LAUNCHER_STEM` | `demo` | the entry point users run |
| `UPGRADER_APP_STEM` | `demo-app` | the executable inside a version folder |

Compile time rather than a config file read at startup, because the launcher has to find the install *before* it can read anything in it. Both binaries are built from one package so they cannot disagree.

`build.ps1` also sets these as real environment variables. Cargo merges `[env]` from every ancestor directory, so a clone sitting inside another cargo project would otherwise inherit that project's values for the same keys — which is a trap worth knowing about if your own app lives inside a larger tree.

## Things worth copying

- **The checksum is optional in the manifest format and supplied here.** `download_asset_verified` refuses a mismatch outright, treating a truncated download exactly like a network failure rather than unpacking it.
- **`Ok(None)` is not an error.** A manifest that has not been published yet is a normal answer, which is why `Fetch::fetch_url` returns an `Option`.
- **The launcher interprets nothing it does not own.** `--version`, `--help` and `--rollback` are answered by the launcher because it is the process the shell waits on; everything else is forwarded to the app unread, so a later app argument needs no launcher change.
- **`build.rs` declares `rerun-if-env-changed`.** `option_env!` is invisible to cargo, so without it, building 1.1.0 then 1.0.0 from one source tree silently reuses the first binary — and the example would demonstrate an update that never happened.
