# wit

The extension ABI — the one contract between the native core and every WASM extension, in any language (ADR-001). It has two halves, and an author outside Rust needs both. They are not equally WASM-specific:

- `extension.wit` matters only to an engine built with the opt-in `wasm` module, which hosts extensions ([ADR-040](../docs/adr/ADR-040-the-wasm-extension-host-is-an-opt-in-module.md)).
- `wire-format.json` is the format of every message on the bus. Native modules exchange exactly these bytes too; an extension is just one more party that has to produce them without sharing the Rust codec.

[`extension.wit`](extension.wit) declares **the calls**:

- **`host-api`** — what an extension may call: `log`, `subscribe`, `publish`, `send`, `request-exit`, and display queries.
- **`world extension`** — what an extension must export: `init`, `shutdown`, `on-tick`, `on-message`.

[`wire-format.json`](wire-format.json) declares **the bytes those calls carry** on core-owned topics: the primitive encodings, the framing rules, and every topic and endpoint with its payload. It is structured data, so tools read it directly. [`wire-format.md`](wire-format.md) is the same specification rendered for people, and machine-readable [conformance vectors](vectors/README.md) sit beside both.

The split is not arbitrary. WIT can describe a `list<u8>` payload but not what is inside it, so a `publish` signature alone tells a guest author nothing about how to build a `gfx/draw-sprite`. The two halves version differently. `extension.wit`'s package version, `bones:extension@<version>`, moves only when the calls change, because wasmtime matches it exactly and a bump breaks every extension. The ABI line — the `abi-v*` tag, and the version of `bones-messages` and `bones-wasm-sdk` — moves whenever either half changes, including a new topic in `wire-format.md`.

## The specification as data

`wire-format.json` is the source of the wire format, and nothing else is edited by hand ([ADR-041](../docs/adr/ADR-041-the-wire-format-is-specified-as-structured-data.md)). `cargo xtask wire-format` renders it as `wire-format.md` and writes [`wire-format.schema.json`](wire-format.schema.json), the JSON Schema it follows, derived from the Rust types that read it. Point an editor at the schema through the file's `$schema` key and it completes and checks every key.

Every entry names the topics or endpoints that share one payload, and the payload is a list of fields, optionally followed by a tagged union:

```json
{
  "topics": ["desktop/result"],
  "payload": {
    "fields": [{ "name": "request_id", "type": "u32", "meaning": "the request this answers" }],
    "union": {
      "tag": "tag",
      "variants": [
        { "tag": 0, "name": "registered", "fields": [{ "name": "registered", "type": "bool" }] },
        { "tag": 1, "name": "failed", "fields": [{ "name": "reason", "type": "str-rest" }] }
      ]
    }
  }
}
```

- **A field's `type`** is a primitive from the Primitives section, a counted list such as `{ "list": "str", "count": "count" }`, or a shape the entry defines under `types`. A defined shape with fields only is written in place: its fields follow on the wire as if listed there.
- **`values`** names what each value of an enumerated `u8` field means.
- **Every Markdown string**, such as `description`, `meaning`, and `notes`, is rendered into `wire-format.md` as written.

Generation fails when a field type names nothing, when a list's count is not an earlier `u16` field, or when a vector lists different field names, or a different order, than the specification for its topic. The vectors come from the codec, so the specification cannot fall behind it.

## What belongs here

Only the contract. No implementation and no generated code: the host side is generated from `extension.wit` by wasmtime's `bindgen!` inside [`bones-module-wasm`](../crates/bones-kernel/bones-module-wasm/README.md), and the guest side by `wit_bindgen::generate!` in [`bones-wasm-sdk`](../crates/bones-wasm-sdk/README.md).

Those two crates are the only places in the repository that name this directory. Extensions do not: a Rust extension depends on `bones-wasm-sdk`, which carries the package and generates the bindings for it.

The Rust *implementation* of the wire format does not belong here either — it lives in [`bones-messages`](../crates/bones-messages/README.md), so the host and Rust guests share one codec. `wire-format.json` is the specification that crate implements, not a description of it; the vectors are what hold the two together.

## Obtaining it

Extension authors outside this repository need `extension.wit` to build against, whatever language they write in.

- **Rust** — do not fetch it at all. Depend on `bones-wasm-sdk`, which carries this package and generates the bindings.
- **Any other language** — take this whole directory from a distribution built by `cargo xtask dist`, which copies it next to the engine binary it belongs to, or from the repository at the revision you are targeting. You need `extension.wit` to generate bindings, `wire-format.md` to build payloads (or `wire-format.json` to generate a codec), and `vectors/` to check that you built them correctly. [examples/extensions/ts-demo](../examples/extensions/ts-demo/README.md) is a worked TypeScript guest whose codec is generated from `wire-format.json` and checked against the vectors.

Pin what you took. Record the `bones:extension@<version>` line from `extension.wit`'s first line alongside your own sources, and re-check it when you update the engine.

## Why the exports sit in an interface

`world extension` exports the `extension-api` interface rather than four bare functions, so a guest's exported symbols are `bones:extension/extension-api#init` and friends.

That qualification is not decoration. A world-level export takes the **bare name** as its core WebAssembly symbol, which put `shutdown` in the same flat namespace as POSIX `shutdown(sockfd, how)` — carried by `libstd` for socket support — and made every guest link with `rust-lld: function signature mismatch: shutdown`. The interface prefix removes that collision and the three it had not hit yet.

## Changing it

This is a published ABI. Any change here breaks every extension built against the old package, including the three projects listed in the root [README](../README.md).

Enforcement is not advisory and not semver-ranged. wasmtime refuses to instantiate a component whose imported interface version differs from the host's at all, so a guest built against `bones:extension@0.2.0` fails to load on a `@0.1.0` engine even when nothing else changed. The error names the mismatch directly:

```text
component imports instance bones:extension/host-api@0.2.0, but a matching
implementation was not found in the linker
```

Structural changes are caught the same way even without a version bump, so a forgotten bump fails safe rather than silently mismatching. One exception is worth knowing: a component that imports nothing at all is version-agnostic, because unused imports are stripped before the check applies.

The practical consequence is that a bump costs every extension in existence, so prefer adding a topic and a typed message in `bones-messages` — that needs no ABI change at all.
