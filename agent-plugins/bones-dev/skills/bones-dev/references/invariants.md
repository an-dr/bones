# What every change keeps

ADR-044 makes AI-assisted product development an architectural property of bones. These are its invariants, as the checks a change must pass. The ADR itself holds the reasoning.

| When you add | It arrives with |
| --- | --- |
| a core topic | its entry in `wit/wire-format.json` and conformance vectors ([wire-format.md](wire-format.md)) |
| a way an extension can fail | a diagnostic, reported once per extension and topic, naming the extension, the cause, and the fix; never a silent drop |
| a capability | a way to observe it from outside the process: a message on the bus, a devtools route, or a log line |
| something visible | a way to capture it: the canvas through `renderer/capture`, or the whole window when it is a native child window such as a web panel |
| a tool or a rule an agent needs | an entry in `agent-plugins/`: a product rule in `bones-user`'s `references/rules.md`, an engine rule here |
| a power an extension could misuse | a permission the composition root can deny (ADR-042) |

## How to tell

- **Headless and repeatable.** Behaviour a product depends on can be proved in `bones-test` with a fixed frame rate: the same steps give the same frames.
- **Nothing to read but the archive.** An agent with only a release archive and a product repository can find it: in the plugin, in `wit/`, or through `bones-cli`.
- **Measured.** A change that makes the agent benchmark (`tests/agent/`) worse is a regression.
