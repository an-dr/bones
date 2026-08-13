# vectors

Conformance vectors for the [core message wire format](../wire-format.md). If you are implementing bones extensions in a language other than Rust, this is what tells you your encoder is right.

## The file

[`vectors.txt`](vectors.txt) is line-oriented on purpose. An implementation checking these may be written in a language whose standard library has no JSON parser, and `bones-messages` — which generates the file — has no dependencies to parse one with either. Splitting a line on spaces is the whole parser.

Each vector is one line of three space-separated fields:

```text
<topic-or-endpoint> <payload-as-lowercase-hex> <field>=<value>;<field>=<value>
```

- Blank lines separate vectors and `#` lines are comments; skip both.
- A payload of `-` is the empty payload, zero bytes.
- Byte-slice values are written as hex themselves, so `bytes=010203` means three bytes.
- The first field is a **topic** for published messages and an **endpoint name** for the three direct-send calls (`web`, `persistence`, `files`).

## Being conformant

For every vector, both directions must hold:

- **Encode** the listed field values and get exactly the listed bytes.
- **Decode** the listed bytes and get exactly the listed field values.

Encoding alone is not enough. A decoder that ignores trailing bytes or accepts a short payload passes an encode-only check and then silently misreads the first message a future version sends.

The vectors cover every core-owned topic, both branches of the optional sprite in `game-core/entity-op`'s spawn, every tag of every tagged union, and the empty-payload cases. They do not cover malformed input: the four failure modes are described in [wire-format.md](../wire-format.md) and are worth testing against, but a vector file of valid payloads cannot express them.

## Where they come from

Generated from `bones-messages`, the same code the engine itself encodes with, and checked back against it by `crates/bones-messages/tests/conformance.rs` on every test run. A vector that disagreed with the engine would fail that crate's own suite, so the file cannot quietly go stale — which is the property that makes it safe to trust over the prose.

Regenerating is deliberate, because an encoding change is an ABI break (ADR-029):

```sh
BONES_WRITE_VECTORS=1 cargo test --test conformance
```

Run from `crates/bones-messages`. Read the resulting diff; it is the list of things you just broke for every extension in existence.
