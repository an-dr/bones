# Changing the wire format

`wit/wire-format.json` is the specification of every core topic and endpoint (ADR-041). `bones-messages` implements it, and the conformance vectors, generated from that implementation, tie the two together.

## First, decide whether the ABI line moves

| Change | ABI line (ADR-029) |
| --- | --- |
| A new core topic | minor: a decoder that does not subscribe never sees it |
| A new variant of a tagged union | minor: a decoder that never receives it is unaffected |
| A changed field, a removed topic, a renumbered tag | major: every extension built before breaks |

A major change costs every extension in existence. Look for an additive alternative first, usually a new topic. A topic added on the current, unreleased line may still change freely.

## Then, in order

1. **Specify it** in `wit/wire-format.json`: the entry with its topics, description, payload fields, and notes, in the right group. The JSON Schema beside it (`wire-format.schema.json`) gives an editor completion for every key.
2. **Implement it** in `crates/bones-messages`: a type with `Message`, `EncodeMessage`, and `DecodeMessage`, and its unit tests. Add the topic to `catalogue::TOPICS` and to `validate`. For a topic a guest can receive, add a case to `Inbound` in `crates/bones-wasm-sdk`.
3. **Add a vector** for every shape, every tag of a union included, in `crates/bones-messages/tests/conformance.rs`, then run `cargo xtask vectors` and read the diff of `wit/vectors/vectors.txt`.
4. **Regenerate** with `cargo xtask wire-format`. It fails when a field name or order disagrees with a vector, or a type names nothing. Then run `cargo xtask ts-codec` for the TypeScript guest.
5. **Use it** where the engine publishes or consumes it, through the typed message, never hand-built bytes.
6. **Record it** in `CHANGELOG.md` under the ABI line, and in `bones-wasm-sdk`'s typed calls if a guest sends it.

`bones-cli` encodes and decodes the new topic from JSON fields with no code of its own, because it reads `wire-format.json` directly.
