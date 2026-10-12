# bones-launcher

Selects an installed version and starts the application inside it. Its only bones dependency is [bones-versioning](../bones-versioning/README.md); it links no kernel, archive handling, or HTTP client.

An application declares its own launcher binary and calls `bones_launcher::run(app_exe, install_dir, args)`. The reference `bones-launcher` binary uses its own directory as the installation root and reads the application stem from `UPGRADER_APP_STEM` at build time.

The launcher forwards application arguments unchanged. Its one selection flag, `--previous`, launches the second-newest installed version and forwards the remaining arguments. It never deletes a version. Application help, version reporting, installation, and rollback belong to the application.

Run `cargo test -p bones-launcher` from the repository root. See [ADR-039](../../docs/adr/ADR-039-versioning-is-pure-the-launcher-launches-and-the-upgrade-module-owns-installation.md) for the boundary.
