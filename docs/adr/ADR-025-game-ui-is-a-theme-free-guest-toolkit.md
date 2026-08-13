# ADR-025: Game UI is a theme-free guest toolkit

## Problem

Games need menus and HUD interaction that render through `gfx/*`, but copying layout, selection, and mouse-scaling code into every extension makes fixes and input behavior diverge. The native egui module serves application-style UI and is not the right presentation layer for an in-world game menu.

## Decision

Bones provides a small Rust crate for WASM guests. It owns logical-canvas geometry, vertical menu layout, keyboard selection, physical-to-logical hit-testing, and owned commands that publish ordinary `gfx/*` messages.

The toolkit contains no host imports, navigation state, theme, fixed labels, or native renderer module. Games provide styling and publish through their own WIT host callback.

## Rationale

Both extensions and demos can reuse deterministic interaction mechanics while remaining visually independent. Keeping the crate guest-side avoids a new runtime protocol and preserves the existing engine-owned rendering boundary.

## Rejected alternatives

- Extend the egui module: game menus would inherit application UI styling and input focus behavior.
- Add a native game-widget renderer: this duplicates `gfx/*` and makes the engine own game-specific policy.
- Share only copied examples: behavior would continue to drift between games.
