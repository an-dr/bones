# Cutting a release

The two version lines are released independently ([ADR-029](../adr/ADR-029-the-two-version-lines-are-the-two-public-surfaces.md)), so the first step decides which of them this release is.

1. **Decide which line moves.** The engine line is `[workspace.package]`'s `version` in the root `Cargo.toml`. The ABI line is `bones:extension@` in [wit/extension.wit](../../wit/extension.wit), plus the explicit `version` in `bones-messages` and `bones-wasm-sdk`. They move independently — never bump one to match the other.
2. **If the ABI moved, regenerate the conformance vectors** — `BONES_WRITE_VECTORS=1 cargo test --test conformance` from `crates/bones-messages` — and read the diff. It is the list of things you just broke.
3. **Run `pwsh test.ps1`.** All gates, both feature sets, green.
4. **Update [CHANGELOG.md](../../CHANGELOG.md)** and commit.
5. **Run `pwsh dist.ps1`** on each platform you are publishing for. Keep each archive and its `.sha256`.
6. **Tag** — `v<version>` for the engine line, `abi-v<version>` for the ABI line.
7. **Attach the archives and their checksums** to the release.

## Two things that will bite

**A tag is immutable.** A fix is a new tag, never a moved one. A git dependency has no checksum a consumer can verify against, so moving a tag silently changes what someone already built against.

**Publish the checksum of the archive you actually uploaded**, not one regenerated later. Zip entries carry timestamps, so two runs of `dist.ps1` over identical sources do not produce byte-identical archives. Reproducible archives are tracked in [roadmap.md](../roadmap.md).

## Before you move the ABI line

An ABI bump breaks every extension built against the previous version — including bumps that change nothing else, because the check is structural rather than semver-ranged. [wit/README.md](../../wit/README.md) has what that costs and how to avoid paying it; the usual answer is to add a topic and a typed message instead, which needs no ABI change at all.
