# Progress

**Flow:** Detailed Auto
**Phase:** COMMIT
**Goal:** One universal, Rust-native build/automation entry point (cargo xtask) for every package bones builds, replacing the duplicated ps1 scripts; plans updated with Android as future work.
**Done when:** cargo xtask test / dist / example reproduce what the ps1 scripts did and test is green; ps1 scripts removed; .gitignore clean; docs and roadmap current with Android planned.
**Constraints:** No new required toolchain beyond Rust + existing C compiler/CMake for SDL. Install nothing automatically. Windows ARM64 primary, portable code. Same outputs as today (dist zip layout, test gates, example .wasm and dist folders).
**Out of scope:** Implementing Android support (planned only); mise/tool provisioning; CI setup; vendor/pubsub-bus changes (stashed edits stay stashed).

## Requests

- Replace the PowerShell build system (test.ps1, dist.ps1, scripts/native-build-env.ps1, every examples/*/build.ps1) with a universal cargo xtask that owns dependency checks, automation logic, and every package build
- Shared prerequisite checks: each missing tool fails fast with where/how to install it; nothing is installed automatically
- Update the plans (docs/roadmap.md): remove outdated items and add Android support as planned future work using the design from this session
- Remove outdated content from the docs, including every reference to the replaced ps1 scripts
- Proper .gitignore entries: add .artifacts/, fix stale comments, drop dead entries

## Iterations

- [ ] **xtask-core** (verified) — xtask crate + cargo alias; prerequisite-check module with install hints (rust targets, C compiler, cmake, ninja preference, MSVC vcvarsall); cargo xtask test ported from test.ps1; delete test.ps1 and scripts/native-build-env.ps1; .gitignore cleanup.
- [ ] **xtask-dist-examples** (planned) — cargo xtask dist (same zip layout + checksum) and cargo xtask example <name> from a table; delete dist.ps1 and all 15 examples/*/build.ps1.
- [ ] **docs** (planned) — README, docs/contributing, example READMEs, code comments referencing ps1; docs/roadmap.md outdated items removed and Android (SDLActivity APK, bones-android crate, cargo xtask android preflight) added; ADR for the xtask build system.

## Questions

- [x] Scope of "Android support": engine runs WASM extensions on an Android device as an APK (shipped engine + bundled extensions), or also embedding as a library in a host Android app? — Whatever is easiest for a simple app: standalone APK.
- [x] Which native modules must work on Android in this branch: renderer+input+audio (SDL3) only, or also egui UI, wry web panels (Android WebView), game-core, persistence? Tray, self-update, single-instance, desktop registration, OS module are assumed out of scope. — Use every module that supports Android out of the box; skip the rest.
- [x] How do extensions arrive on device: bundled into APK assets only, or also loaded from app storage (enabling hot reload over adb push)? — Bundled in the APK.
- [x] Target ABIs and min API level (suggest arm64-v8a only, API 26+)? Is there a physical device or emulator to verify on, and is Android SDK/NDK installed here? — Agreed: arm64-v8a, API 26+.
- [x] Definition of done: APK builds via one pwsh script and hello-demo + one sprite example run on device — or more? — Fine: one pwsh script builds the APK; hello-demo and a sprite example run on device.
- [x] Android SDK, NDK, and JDK 17 are not installed and no Android Rust target is present. May I install them (Android command-line tools + NDK r27 + platform 35 under %LOCALAPPDATA%\Android\Sdk, Temurin JDK 17 via winget, rustup target aarch64-linux-android, cargo-ndk)? And verify on a physical phone over adb, or an emulator (arm64 emulator on Windows ARM is limited)? — Do not install. android.ps1 runs a preflight that checks each prerequisite (JDK 17, ANDROID_HOME/SDK, platform 35, build-tools, NDK, rustup aarch64-linux-android target, cargo-ndk, adb/device for install) and fails with a clear message and where/how to install it. Device verification is done by the user after installing.

_Last updated: 2026-10-17T17:19:30.4053898Z_