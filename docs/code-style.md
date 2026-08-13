# Code style and file structure

Precise, code-level conventions for Rust source in this repo: how to lay out files and modules, and what a crate's README should say. This is the one exception to [index.md](index.md)'s "behavior and boundaries, not code" altitude rule — file layout *is* the subject here, not a side effect of it. The root [AGENTS.md](../AGENTS.md) states the underlying rules (one type per file, tests out of line); this document is the precise pattern that satisfies them, kept in sync with whatever the codebase actually converged on.

## One type per file

Each `struct`/`enum`, its impls, and any private helpers that exist only to serve it get their own file, named after the type in `snake_case` (`DrawSprite` → `draw_sprite.rs`).

**Exception, stated inline**: two types stay in one file only when one is a private implementation detail of the other — never constructed or named outside that file, no independent identity of its own. State the reason as a comment above the private type. Real examples in this codebase:

```rust
// crates/bones-engine/bones-kernel/src/wasm_extensions/host.rs
// `State` and `Host` stay in one file rather than splitting further: `State`
// is purely `Host`'s internal store data (implements the WIT imports and
// `WasiView` only for `Host`'s own use), never meaningful on its own.
struct State { /* ... */ }
pub struct Host { /* ... */ }
```

```rust
// crates/bones-engine/bones-module-renderer/src/renderer.rs
// `State` stays in this file rather than splitting further: it's purely
// `Renderer`'s own internal store (the `SendWrapper` payload), never
// constructed or named outside this file, never meaningful on its own.
struct State { /* ... */ }
```

If the second type has any independent identity — constructed elsewhere, matched on elsewhere, re-exported and used standalone — it does not qualify for this exception and gets its own file, even if the two are closely related conceptually.

**Macro-generated type families** are the other stated exception: when a `macro_rules!` generates near-identical types (e.g. `KeyDown`/`KeyUp` from one `key_message!` macro), the macro definition and its invocations stay together in one file named after the concept, not the individual types:

```rust
// crates/bones-messages/src/input/key.rs
macro_rules! key_message { /* ... */ }
key_message!(KeyDown, "input/key-down", "A keyboard key was pressed.");
key_message!(KeyUp, "input/key-up", "A keyboard key was released.");
```

## The submodule shape

When a file `foo.rs` needs to split (multiple types, or has grown past a size where one type per file makes sense), it becomes a directory module:

```text
src/
├── foo.rs           # mod declarations + pub use re-exports + crate-level //! doc
└── foo/
    ├── one_type.rs
    ├── another_type.rs
    ├── command.rs    # only if foo.rs has a dispatch enum (see below)
    └── tests.rs       # #[cfg(test)] mod tests; declared in foo.rs
```

`foo.rs` itself becomes pure plumbing — no logic, just:

```rust
//! One-line crate-level doc for what this module is.

mod another_type;
mod one_type;

pub use another_type::AnotherType;
pub use one_type::OneType;

#[cfg(test)]
mod tests;
```

Worked example — `crates/bones-messages/src/gfx.rs`:

```rust
//! Typed `gfx/*` draw commands shared by extensions and the renderer.

mod clear;
mod command;
mod draw_circle;
mod draw_line;
mod draw_rect;
mod draw_sprite;
mod draw_text;
mod load_sprite;
mod set_camera;

pub use clear::Clear;
pub use command::Command;
pub use draw_circle::DrawCircle;
pub use draw_line::DrawLine;
pub use draw_rect::DrawRect;
pub use draw_sprite::DrawSprite;
pub use draw_text::DrawText;
pub use load_sprite::LoadSprite;
pub use set_camera::SetCamera;

#[cfg(test)]
mod tests;
```

Every `pub` item that existed on the flat file must still resolve at the same path after the split — the re-exports carry that, not the caller.

## The `command.rs` dispatcher file

When a module has a topic-dispatch enum matching over every message type it defines (the `gfx::Command` / `audio::Command` pattern: "decode by exact topic, `Ok(None)` for unknown"), that enum and its `decode` method get their own `command.rs`, importing every sibling type via `use super::{...}`:

```rust
// crates/bones-messages/src/gfx/command.rs
use crate::{DecodeError, DecodeMessage, Message};
use super::{Clear, DrawCircle, DrawLine, DrawRect, DrawSprite, DrawText, LoadSprite, SetCamera};

pub enum Command<'a> { /* one variant per sibling type */ }
impl<'a> Command<'a> {
    pub fn decode(topic: &str, payload: &'a [u8]) -> Result<Option<Self>, DecodeError> { /* ... */ }
}
```

Not every module needs one — only add it when there's a real dispatch enum, not preemptively.

## Tests: always out of line

`foo.rs`'s `#[cfg(test)] mod tests { ... }` moves to `foo/tests.rs`, included as a bare declaration:

```rust
#[cfg(test)]
mod tests;
```

`tests.rs` itself starts with `use super::*;` plus whatever else the tests need (`crate::{...}` imports for traits not re-exported by the parent), then the `#[test]` functions verbatim — no renaming, no restructuring beyond the move itself.

For a flat file with no submodule split (a true single-type file), the same rule still applies: move the test module to `<name>/tests.rs` next to it, which promotes the file to a one-entry directory module purely to hold the test file. This is expected and correct, not over-fragmentation.

## READMEs

Every crate (anything with its own `Cargo.toml` under `crates/`) has a `README.md` at its root. Keep it short and behavioral, matching the tone of existing ones (`crates/bones-engine/bones-module-renderer/README.md`, `crates/bones-engine/bones-kernel/README.md`): what the crate is for, its wire contract or public shape if it has one, and any non-obvious constraint a caller needs to know before depending on it — not a restatement of what the code already says via doc comments.

## What this document is not

This is not architecture, design, or decision documentation — that's [architecture.md](architecture.md), `design/`, and `adr/`. Nothing here should ever need to change because of a refactoring that keeps the same file-layout conventions; it changes only when the conventions themselves change (a new exception discovered, a new pattern adopted).
