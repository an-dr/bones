# Cutting a release

The two version lines are released independently ([ADR-029](../adr/ADR-029-the-two-version-lines-are-the-two-public-surfaces.md)), so the first step decides which of them this release is. A release publishes source, not binaries ([ADR-046](../adr/ADR-046-releases-publish-source-not-binary-archives.md)).

1. **Decide which line moves.** The engine line is `[workspace.package]`'s `version` in the root `Cargo.toml`. The ABI line is `bones:extension@` in [wit/extension.wit](../../wit/extension.wit), plus the explicit `version` in `bones-messages` and `bones-wasm-sdk`. They move independently — never bump one to match the other.
2. **If the ABI moved, regenerate the conformance vectors** — `cargo xtask vectors` — and read the diff. It is the list of things you just broke.
3. **Run `cargo xtask test`.** All gates, both feature sets, green. CI runs it too, on every platform, but a red CI on a release commit is a release you have to redo.
4. **Update [CHANGELOG.md](../../CHANGELOG.md)** and commit.
5. **Tag and push the tag** — `v<version>` for the engine line, `abi-v<version>` for the ABI line.

A pushed `v*` tag is the whole manual release step: [CI](../../.github/workflows/ci.yml) tests the tagged commit on every platform, builds all six variants — Windows x64 and ARM64 with web panels, Linux x64 and ARM64 each with and without — and publishes a GitHub Release whose notes are the tag's `## engine <version>` section of `CHANGELOG.md`. It uploads no binaries; GitHub attaches the source archives. An `abi-v*` tag publishes nothing.

A user gets a release through `bones-develop`, which builds the engine at its tag ([ADR-047](../adr/ADR-047-bones-develop-installs-engines-by-tag.md)); `cargo xtask dist` at the tag, adding `--features web` for the web variant, builds the same for the machine it runs on.

## Two things that will bite

**A tag is immutable.** A fix is a new tag, never a moved one. A git dependency has no checksum a consumer can verify against, so moving a tag silently changes what someone already built against.

**The changelog section names the release.** The release notes come from the tag's `## engine <version>` heading; write it before tagging by hand, or the release job fails.

## Before you move the ABI line

An ABI bump breaks every extension built against the previous version — including bumps that change nothing else, because the check is structural rather than semver-ranged. [wit/README.md](../../wit/README.md) has what that costs and how to avoid paying it; the usual answer is to add a topic and a typed message instead, which needs no ABI change at all.
