# bones-upgrader

Self-update for a bones desktop application: manifest fetch, version comparison, version folders, a permanent launcher, staged installs and rollback.

The crate knows nothing about the application it updates. Four names — the install directory, the environment override, the launcher stem and the app stem — arrive at compile time through `option_env!`, so a host states them once in its own `.cargo/config.toml` `[env]` block and both its executables agree without either parsing configuration at runtime. Unset, they fall back to neutral placeholders.

The one capability it asks of a host is `Fetch::fetch_url`. Everything else is filesystem and version arithmetic, so a host with no clipboard and no file pickers can still use it, and a test satisfies it with a map of canned responses.

It is called `upgrader` because Windows auto-elevates any executable whose filename contains `update`, `install`, `setup` or `patch` — test binaries included, which is how this was found ([ADR-033](../../docs/adr/ADR-033-self-update-is-a-crate-and-its-binaries-avoid-installer-words.md)). The same is true of a binary a host builds from it.

## The two halves

An application cannot replace its own running entry point, so the mechanism ships as two binaries: the launcher, which lives at the install root under the name users type and never changes, and the app, which lives in a version folder the launcher picks. `bones-launcher` is the reference launcher; a host normally declares its own `[[bin]]` under its own name, calling `launcher::run`.
