# Roadmap

Add future work here only when it has a demonstrable completion artifact; completed work belongs in git history.

## Android

bones runs on Android as a library inside an application's own APK: arm64-v8a, API level 26 and up. There is no Android release of the `bones` executable. An Android user installs apps rather than dropping files into a folder, and since extensions are optional ([ADR-040](adr/ADR-040-the-wasm-extension-host-is-an-opt-in-module.md)), the binary-plus-extensions model is not one the engine has to offer there. Other ABIs and Play Store signing are out of scope for the first version.

- **Packaging.** SDL3 ships Android support — a Java `SDLActivity` plus a native `SDL_main` — and the engine already sits on SDL3, so an application's APK is SDL's activity loading the application as `libmain.so`. A `bones-android` crate provides the `SDL_main` glue around the same `Kernel` builder an embedder uses on the desktop.
- **Modules.** Only what works on Android out of the box: renderer, input, egui UI, kira audio, game-core, and persistence. wry web panels stay desktop-only — wry needs its own Android activity and cannot share SDL's. Tray, self-update, single-instance, desktop registration, and the OS module are desktop concepts. The `wasm` module is the application's choice, as on the desktop.
- **Portability work first.** The kernel and the enabled modules must compile for `aarch64-linux-android`, which means gating the desktop-only dependencies (single-instance, the file watcher, desktop paths) behind `cfg`.
- **Build.** `cargo xtask android` builds `libmain.so` with `cargo-ndk` and produces a debug APK from an example's Gradle project, optionally installing it over `adb`. Its prerequisite checks cover JDK 17, the Android SDK (platform 35 and build-tools), the NDK, the `aarch64-linux-android` Rust target, `cargo-ndk`, and `adb`, each failing with where to install it; nothing is installed automatically.

Completion artifact: `cargo xtask android` producing an APK of a native example application from a clean clone on a machine with the prerequisites, running on an arm64 device.

## An Android example that manages its extensions

Once the library runs on Android, an example APK showing an application that hosts extensions there: it enables the `wasm` module, bundles extensions in its assets or lets the user add them, and gives the user a screen to enable, disable, and reload them. It is an example of what an application builds on the existing runtime activation (startup allow-list, `core/extensions/load`, `unload`, `reload`, and `core/lifecycle`), not an extension manager inside the engine.

Completion artifact: the example APK on an arm64 device, adding an extension at runtime and toggling it from its own screen.

## Per-extension topic permissions

Every extension may publish to and subscribe to any topic: the sandbox limits what an extension can reach in the OS, not what it may say on the bus. That is right for an application's own extensions and too generous for third-party plugins, where an application may want "this plugin may not use `os/*`".

Because the bus is universal, permissions belong in the `wasm` module, as a filter on each extension's endpoint, declared by the application at composition time. No change to the bus or the ABI is needed.

Completion artifact: a test in which an extension denied a topic prefix has its publish to that prefix rejected and logged, while an extension without the restriction still reaches it.

## Supply-chain checks and the rest of the platform matrix

[CI](../.github/workflows/ci.yml) runs `cargo xtask test` on Windows and Linux, x64 and ARM64, and builds the release archives there. Three gaps remain, and they are separable:

- **A dependency advisory and licence scan.** It matters more than usual here because the engine links its whole dependency tree statically and ships generated third-party notices.
- **An MSRV policy**, or an explicit, documented decision that the policy is current-stable-only. Today there is neither.
- **Web panels on Linux, and macOS.** The wry web-panel tests are gated to Windows, so the Linux `-web` archives are built but their panels have never been exercised by a test. macOS has no runner at all.

Completion artifact: CI running an advisory and licence scan on every push, the wry tests running on Linux, and a `rust-version` in `[workspace.package]` or a documented policy that CI enforces.

## Reproducible release archives

`cargo xtask dist` writes a SHA256 beside each archive, which verifies a download but not a build: zip entries carry modification times, so two runs over identical inputs produce different hashes. A consumer cannot check that the archive they downloaded is the one this repository's source produces.

Completion artifact: two runs of the CI build job over the same commit producing byte-identical archives, and the checksum published from source rather than from the upload.

## API documentation on the ABI-line crates

`bones-kernel` enables `missing_docs`, so the embedder-facing surface cannot grow an undocumented public item. `bones-messages` and `bones-wasm-sdk` do not, and between them have roughly two hundred undocumented public items — mostly message fields and generated bindings.

The message fields are not undocumented in substance: [wit/wire-format.md](../wit/wire-format.md) specifies every one of them, for every language rather than only Rust, which is where an extension author should be reading. What is missing is the rustdoc a Rust author sees at the call site, and the lint that would keep it honest.

Completion artifact: `missing_docs` enabled on both crates with the gap closed, and the message documentation generated from or checked against the wire-format specification rather than written twice.

## Panels that paint while the frame thread is busy

A web panel opened just before a long step on the frame thread — loading a large extension at startup — stays blank until the step ends. Measured on Windows with a test that opens a panel and then blocks the frame thread for 3 s: the page's script runs only after the thread resumes, whether the page is served through the custom protocol, a WebView2 virtual-host folder mapping, a `data:` URL, or a loopback HTTP server, and whether or not the window had settled first. Serving the page differently does not fix it. Since even a `data:` URL waits, the likely cause is the webview's child window waiting on the thread that owns its parent window, not the page request.

Two ways out remain, and each is a design decision rather than a patch: move long work such as extension instantiation off the frame thread, or host the webview by composition (WebView2's composition controller) rather than as a child window, which wry does not offer.

Completion artifact: a test that opens an `Html` panel, blocks the frame thread for 3 s, and receives a timestamp from the page earlier than the end of the block.
