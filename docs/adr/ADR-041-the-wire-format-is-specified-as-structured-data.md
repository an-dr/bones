# ADR-041: The wire format is specified as structured data

## Problem

The message contract had three forms: prose in `wit/wire-format.md`, Rust types in `bones-messages`, and the conformance vectors. A person can read the first, Rust can call the second, and a test can run the third. A tool that is not written in Rust cannot read any of them as structured data. That covers an MCP server encoding a message an agent asked for, a code generator writing a codec in another language, and an engine suggesting the topic a typo meant.

Extracting structure from the Markdown tables works for plain field lists but not for the rest. Tagged unions, counted sequences, and enumerated values are only conventions in prose, and every consumer re-derives them, differently.

## Decision

- **`wit/wire-format.json` is the specification.** It holds every core topic and direct-send endpoint, grouped as before. For each one it gives the description, the payload as a field list followed optionally by a tagged union, and any notes. It also holds the prose sections that apply to all messages: scope, versioning, primitives, framing, decoding failures, and conformance. Field types are primitives or counted lists, a `u8` field may name its values, and shapes used in more than one place are named types.
- **`wit/wire-format.md` is generated from it** by `cargo xtask wire-format`, together with `wit/wire-format.schema.json`. The schema is derived from the same Rust types that read the specification, so it cannot drift from them. Neither generated file is edited by hand.
- **Generation checks the specification against the codec.** For every topic, the field names in the specification must equal, in order, the field names in that topic's conformance vector. A union's fields come after its tag, and a count before its list may or may not appear. The vectors come from `bones-messages`, so a renamed, added, or reordered field that the specification missed fails the build. The workspace tests regenerate both files in memory and fail when the committed ones differ.
- **Tools read the JSON.** `bones-cli`, the TypeScript code generator, and the consistency tests read `wire-format.json` directly.
- **It ships with the ABI.** It sits beside `extension.wit`, moves on the ABI line, and every distribution that carries `wit/` carries it.

## Rationale

A structured source turns into prose without loss, but prose turns into structure only through conventions every reader must share. With the JSON as the source, unions, sequences, and enumerated values are data that every tool reads the same way. The explanations survive as Markdown strings in the same file. The vectors already tie the codec to observed bytes, and checking field names against them ties the specification to the codec as well.

## Rejected alternatives

- **Keep the Markdown as the source and extract a catalogue from its tables.** Every consumer of a union or a sequence would re-implement the same prose conventions.
- **Generate from `bones-messages` with a schema trait on every message.** It needs a second description of every field beside its encoder, in Rust only, and makes the code rather than the specification the authority.
- **Write both the JSON and the Markdown by hand.** Two copies of the contract would drift, with nothing to say which one is right.
