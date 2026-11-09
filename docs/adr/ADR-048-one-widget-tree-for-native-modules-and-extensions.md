# ADR-048: One widget tree for native modules and extensions

Extends [ADR-005](ADR-005-egui-ui-layer.md), whose decision stands, and replaces its sentence that the vocabulary stays at about a dozen widgets.

## Problem

The `ui/*` vocabulary has three widgets in a flat list drawn as one floating window per extension. A native application that wants a real desktop interface gets nothing from it, so it owns an egui context of its own and writes its own widgets, theme, and title bar; every such application repeats that work. Products should build interfaces from widgets bones ships.

## Decision

- **One widget tree.** An interface is a tree of layout containers and widgets, rooted in a *Window* (a floating panel, as before) or a *Screen* (the whole window, with top, side, bottom, and central panels; one owner at a time).
- **Two doors, one model.** A native module hands the tree to the ui module in Rust, with no encoding, through a `ui` service the module publishes ([ADR-031](ADR-031-native-modules-reach-each-other-only-through-services.md)). An extension publishes the same tree encoded on `ui/spec`. Both draw identically.
- **View and events.** The owner rebuilds the tree from its state every frame. Interactive widgets carry an owner-assigned id; the ui module reports what happened as events naming that id: returned to a native owner, published as `ui/*` to an extension owner. State the owner does not model, such as scroll position, focus, and animation progress, is kept by the ui module per widget id.
- **Open for extension.** The ui module draws every widget through a registry of widget kinds. The standard widgets are registered kinds like any other; a native application registers its own kinds when composing the engine, and both doors can place them. Adding a kind never changes the ui module.
- **A broad standard set.** bones ships the widgets ordinary desktop applications need: text, buttons, inputs, selection, images, icons, cards, progress, toasts, media tiles, drag and drop, motion, and the title bar. The set grows as versioned additions.

The `ui/spec` encoding changes incompatibly, which is an ABI major release.

## Rationale

- Each application stops reinventing widgets, and a fix in one widget reaches every product.
- One model keeps extensions and native applications equal, which keeps the agent tooling, captures, and tests the same for both.
- The registry gives products an escape hatch without forking the module, and keeps bones' own widgets on the same footing as theirs.
- Extensions still name widgets, never egui objects, so the toolkit behind `ui/*` can change without touching them.

## Rejected alternatives

- **Everything through encoded messages, native modules included**: one path, but native screens would pay for an encode and decode every frame for nothing.
- **A native Rust widget library beside the small message vocabulary**: quickest, but extensions and native applications would get different widgets.
- **Callbacks in the tree**: closer to plain egui, but extensions cannot send closures, so the doors would differ.
- **Keep the encoding backward compatible**: the flat list cannot express layout, and no shipped product depends on it enough to justify two encodings.
