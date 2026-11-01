# ADR-046: Releases publish source, not binary archives

## Problem

Every engine release published six per-platform archives built by CI, each with fixed contents: binaries, ABI, agent plugin, notices. A change to shipped non-code material, such as the agent plugin, raised the question of whether to rebuild six binaries or repackage old ones, and repackaging needed release metadata to keep the version an archive reports apart from the version its binaries were built from. Product authors already build their extensions with Rust and Cargo, and an embedder consumes bones only by git tag.

## Decision

A release is a `v*` tag and its notes. CI publishes a GitHub Release for the tag whose text is the tag's `CHANGELOG.md` section, with only the source archives GitHub attaches to every release. CI still builds every platform and variant in release mode on every push, as a check, and uploads nothing.

A user builds the engine and `bones-cli` at a tag with `cargo xtask dist`, which writes the `dist/` layout an archive held. This supersedes the part of ADR-043 that ships `bones-cli` in every release archive; `bones-cli` is still built beside `bones` and finds it as its sibling. A tool that becomes the main way to install bones or add it to a project is planned in the [roadmap](../roadmap.md).

Agent plugin changes get no release mechanism of their own: they reach users with the next engine tag. Delivering agent material without waiting for a tag belongs to the planned tool, not to the release.

## Rationale

A source release has no contents to choose: whatever the tag holds is the release, so no change needs a rule for what to rebuild or repackage. A binary built at a tag reports that tag's version and carries that tag's plugin, so no release metadata is needed.

## Rejected alternatives

- Repackaging the previous release's binaries with new agent material: it needed checked downloads of earlier assets and a second version recorded per archive, to save six builds.
- Rebuilding and uploading six archives for every release: it keeps CI output as the user's way in, with its checksums, platform names, and the question of what each archive contains.
- An automatic patch release for every agent plugin change: it tags whatever else landed since the previous tag, features included, under a patch number.
- A separate version line for agent material: one engine checkout would have two release identities.
