# ADR-038: The assembled system is the kernel, built on the kernel infrastructure

## Problem

"Engine" named two different things. bones as a whole is described as an engine for apps and games, and the one library an embedder depends on was also `bones-engine`, with an `Engine` type that builds and runs the composed system. A reader could not tell from the word whether a sentence was about the product or about one crate inside it. Explaining single instance made the cost visible: "the claim runs before the engine exists" read as a statement about the product, when it meant the composed object.

"Kernel" had the opposite problem. It named the always-present core — bus, frame loop, platform, extension hosting — which is accurate, but it left no good name for the assembled whole that loads optional modules and hosts sandboxed extensions, which is what an operating system calls the kernel.

## Decision

Rename both, keeping every package boundary of [ADR-030](ADR-030-package-structure-follows-consumer-use-cases.md):

| Was | Is | What it is |
| --- | --- | --- |
| `bones-engine`, `Engine`, `BuiltEngine` | `bones-kernel`, `Kernel`, `BuiltKernel` | The public library an embedder depends on, and the assembled system it builds: the kernel infrastructure, the modules composed into it, and the extensions it hosts |
| `bones-kernel` | `bones-kernel-infra` | The always-present core: bus, logging, contract, platform, extension loading and supervision, the frame loop |
| `crates/bones-engine/` | `crates/bones-kernel/` | The directory holding the kernel crate, the kernel infrastructure, and the native modules |

The shipped executable keeps its package and binary name, `bones`. "Engine" remains the word for the product and for the version line ADR-029 names; no code name uses it.

## Rationale

**The operating-system picture fits, and readers already know it.** A kernel is the trusted, running core that loads modules and serves sandboxed user programs through a narrow interface. That is exactly the assembled bones system: native modules loaded into it, WASM extensions reaching it only through messages.

**The always-present part is named for what it is to the kernel: its infrastructure.** `bones-kernel-infra` says whose it is and that it sits underneath, so "a module depends on `bones-kernel-infra`" reads correctly — modules use the kernel's foundation, not the kernel itself, which is also why there is no cycle. Names that were considered and rejected: "core", because "kernel" and "core" mean nearly the same thing and would reintroduce two words for one idea; a "main module", because "module" already names the optional plug-ins and the infrastructure is the one part that is not one; and a metaphor such as "backbone", which every reader would have to learn before it told them anything.

**Both renames, not one.** Renaming only the library to `bones-kernel` would have collided with the existing core crate, and calling the core "kernel" while the whole was something else kept two names for one idea in readers' heads. Moving each name one level up removes the overlap entirely.

**The boundaries do not change.** The kernel crate still depends on the modules and the modules on the kernel infrastructure, never the reverse, so the dependency cycle ADR-030 measured cannot return. Only names move.

## Consequences

- A breaking change for every embedder: the dependency, the crate path in code, and the builder type all change. The engine line moves to 2.0.0. The ABI line does not move; `bones-messages` and `wit/` are untouched.
- ADR-030 and earlier ADRs keep their wording; read their `bones-engine` as `bones-kernel` and their "kernel" as the kernel infrastructure.
- Bus sender names are unchanged: the kernel infrastructure still stamps `engine` on the lifecycle messages it publishes, because that string is visible to guests.
