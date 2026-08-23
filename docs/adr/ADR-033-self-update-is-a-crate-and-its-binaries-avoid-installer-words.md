# ADR-033: Self-update is a crate, and its binaries avoid installer words

## Problem

A desktop application that ships outside a store updates itself, and nothing about doing so is specific to one application: find a manifest, compare versions, download an asset, verify it, unpack it beside the running copy, and hand over to it on the next start. Every bones application that ships to users needs that, and until now each would have written it again.

The mechanism also has one property that is not obvious from its description and costs a day to rediscover. Windows carries a legacy installer-detection heuristic that inspects an executable's filename: a binary whose name contains `update`, `install`, `setup`, or `patch` is assumed to be an installer and auto-required to elevate. The prompt appears with no manifest asking for it and no code requesting it, and it fires for a test binary as readily as a shipped one, so a crate named `updater` cannot run `cargo test` on Windows without a UAC prompt per test binary.

That is a naming constraint on a crate meant to be reused, which is what makes it a decision rather than a detail: a consumer who names their launcher `myapp-update` inherits the same behaviour and will not connect the prompt to the name.

## Decision

Self-update ships as `bones-upgrader`, a crate on the engine version line, with the four host-specific names supplied at compile time through `option_env!` rather than spelled into the mechanism.

It is called `upgrader` for the reason above, and the same heuristic reaches any binary built from it — including the launcher a host declares under its own name, as the reference `bones-launcher` shows.

A host states its identity — install directory, environment override, launcher stem, app stem — in its own `.cargo/config.toml` `[env]` block. Unset, the crate compiles against neutral placeholders (`.bones-app`) so it is usable before anyone configures it, and so an unconfigured release is visibly rather than silently wrong.

## Rationale

Compile-time identity rather than a runtime config file is what lets the two executables agree without either parsing anything. The launcher and the app are separate binaries that must resolve the same install directory; a runtime file is a third thing to keep in sync and a fourth thing to fail to find. `option_env!` makes disagreement impossible: they were built from the same `[env]` block or they were not built together at all.

The name is recorded here rather than only in a comment because a comment explains only the file it sits in, and a consumer choosing a name for their own launcher reads neither. It was measured rather than assumed: the elevation prompt appeared on `cargo test` for a crate named `updater`, before any of this was written down.

Placeholder defaults rather than a compile error for an unconfigured host is the same tradeoff `UPGRADER_LAUNCHER_ICON` already makes in `build.rs`: a consumer evaluating the crate should be able to build it, and an application that reaches release without setting its own names installs into a directory whose name says so.

## Rejected alternatives

- **A runtime configuration file for the identity.** Rejected because the launcher would have to find and parse it before it knows where anything lives, which is the bootstrapping problem the identity exists to solve.
- **Keep the name `updater` and add an application manifest declaring `requestedExecutionLevel=asInvoker`.** This suppresses the heuristic for a shipped binary but not for `cargo test`'s binaries, which no manifest reaches. It also spends a manifest on a problem a name avoids for free.
- **Leave self-update in each application.** Honest while there was one application, and wrong as soon as there were two: the version arithmetic and the entry-point replacement are the parts most likely to be got subtly wrong, and least likely to differ between hosts.
- **Compile-error when the identity is unset.** Safer for release, worse for evaluation, and the failure it prevents is visible in the install path anyway.
