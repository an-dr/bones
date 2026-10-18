# Cutting a release

The two version lines are released independently ([ADR-029](../adr/ADR-029-the-two-version-lines-are-the-two-public-surfaces.md)), so the first step decides which of them this release is.

1. **Decide which line moves.** The engine line is `[workspace.package]`'s `version` in the root `Cargo.toml`. The ABI line is `bones:extension@` in [wit/extension.wit](../../wit/extension.wit), plus the explicit `version` in `bones-messages` and `bones-wasm-sdk`. They move independently — never bump one to match the other.
2. **If the ABI moved, regenerate the conformance vectors** — `cargo xtask vectors` — and read the diff. It is the list of things you just broke.
3. **Run `cargo xtask test`.** All gates, both feature sets, green. CI runs it too, on every platform, but a red CI on a release commit is a release you have to redo.
4. **Update [CHANGELOG.md](../../CHANGELOG.md)** and commit.
5. **Tag and push the tag** — `v<version>` for the engine line, `abi-v<version>` for the ABI line.

A pushed `v*` tag is the whole release step: [CI](../../.github/workflows/ci.yml) tests the tagged commit on every platform, builds the six archives — Windows x64 and ARM64 with web panels, Linux x64 and ARM64 each with and without — and publishes them with their `.sha256` files as a GitHub Release. The README's [release archives](../../README.md#release-archives) section says which one a user needs. An `abi-v*` tag publishes nothing: the ABI ships inside every engine archive.

To build an archive by hand, run `cargo xtask dist`, adding `--features web` for the web variant; it builds only for the machine it runs on.

## Two things that will bite

**A tag is immutable.** A fix is a new tag, never a moved one. A git dependency has no checksum a consumer can verify against, so moving a tag silently changes what someone already built against.

**The published checksum is the uploaded archive's**, never one regenerated later: CI publishes the `.sha256` written beside each archive it uploads. Zip entries carry timestamps, so two runs of `cargo xtask dist` over identical sources do not produce byte-identical archives. Reproducible archives are tracked in [roadmap.md](../roadmap.md).

## Before you move the ABI line

An ABI bump breaks every extension built against the previous version — including bumps that change nothing else, because the check is structural rather than semver-ranged. [wit/README.md](../../wit/README.md) has what that costs and how to avoid paying it; the usual answer is to add a topic and a typed message instead, which needs no ABI change at all.
