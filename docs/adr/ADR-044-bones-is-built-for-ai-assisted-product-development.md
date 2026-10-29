# ADR-044: bones is built for AI-assisted product development

## Problem

More and more of the code in products built on bones is written by AI agents. An agent in a product repository has no engine source, no maintainer to ask, and often no way to look at the window. It needs to find the contract, start a project, build, see what the running engine does, look at what it draws, and prove a change works, all on its own. Each of these was possible before, but only for someone who could read the engine's code.

The features that make this work are spread over many components: the wire format as data, the devtools channel, the test harness, the diagnostics, the permissions, the agent plugins, and the developer tools. Each one can quietly regress when a later change does not know it is part of a whole. Examples: a new topic left out of the specification, a module that drops a bad payload silently, a capability with no way to observe it from outside.

## Decision

Support for AI-assisted product development is an architectural property of bones, held to the same standard as the sandbox and the bus. Every change is judged against these invariants:

1. **The contract is data.** Every core topic is specified as data in `wit/wire-format.json`, rendered for people as `wit/wire-format.md`, and checked against conformance vectors (ADR-041). A tool or an agent looks a message up rather than guessing it.
2. **The running engine is observable from outside, as data and as pixels.** Anything an extension or module does can be read from outside the process: bus traffic, extension state, the log, and the frame, both as draw commands and as the pixels the real renderer produced. Data is what a test compares. Pixels show what data cannot: a screenshot for documentation, a design judged by eye, proof that the renderer drew what the commands said. Pixels include everything the window shows, web panels too, and the canvas alone needs no display: it comes from an offscreen window as readily as from a visible one.
3. **Behaviour is provable without a display, and repeatably.** A built extension runs in `bones-test` at a fixed frame rate, headless or with the real renderer in an offscreen window. The same steps give the same frames. Its picture compares as text, and as pixels where text cannot say enough.
4. **Failures name their cause and their fix.** A mistake an extension makes is reported where it is detected: a malformed payload, a topic nobody hears, a refused permission, an exceeded budget. The report names the extension, what went wrong, and what changes it, and it comes once rather than every frame. Nothing a guest does wrong is dropped silently.
5. **Context ships with the artifact.** Everything bones offers an agent is an [Agent Plugins](https://agent-plugins.org/) package in `agent-plugins/`, the one home of AI-specific material. Every release archive carries the product plugin, and every new project gets the files agents read generated from it, so the rules an agent cannot infer arrive wherever it works. The tools an agent drives come with them: `bones-cli`, and its MCP server (ADR-043).
6. **Untrusted code is bounded by the composition root.** Code nobody has reviewed can be kept away from powerful topics and endpoints without changing it (ADR-042).
7. **Progress is measured.** The agent benchmark (`tests/agent/`) runs against each release, and a change that makes agents do worse is a regression.

In practice:

- a new core topic arrives with its specification entry and vectors;
- a new failure path arrives with a diagnostic;
- a new capability arrives with a way to observe it, and a new visual capability with a way to capture it;
- a rule an agent would get wrong arrives in the `bones-user` plugin's rules.

## Rationale

The bus was already the right foundation. Every effect is a message, so it can be recorded, replayed, and inspected. Extensions never render, so their output is data, and the renderer can read back what it drew, so the pixels are available too. The sandbox means an agent's mistake is quarantined rather than a crash. Naming the property, and stating the invariants that keep it, turns those accidents into guarantees a reviewer can check. It also explains why ADR-041, ADR-042, and ADR-043 exist: they are its parts, not unrelated conveniences.

## Rejected alternatives

- **Treat agent support as tooling outside the architecture.** The tools would keep working while the engine drifted away from them, through new topics outside the catalogue, silent drops, and invisible state. Those failures come from engine changes, so the engine's own rules must prevent them.
- **Optimise for one agent product.** MCP, `AGENTS.md`, skills, and Agent Plugins serve the agents in use today. The invariants above are what any of them needs, and they outlive any one protocol.
