# Use cases: widget interfaces

bones draws application interfaces from widgets it ships, so a product never writes its own widget code to get an ordinary desktop screen ([ADR-048](../adr/ADR-048-one-widget-tree-for-native-modules-and-extensions.md)). Both kinds of product in [bones-develop.md](bones-develop.md) use the same widgets: an extension product describes them in messages, and a native application hands them to the engine in Rust.

## Building an interface

### UC-UI-01: Build a desktop application screen from bones widgets

- **Actor**: application developer, AI agent.
- **Goal**: a whole-window application interface (title bar, panels, lists, image grids, forms, dialogs) without writing widgets.
- **Flow**: each frame, build the screen from the application's state out of bones widgets and layouts, then handle the events bones returns, each naming the widget that produced it.
- **Success**: the window shows the screen, and clicks, edits, selections, and drops arrive as events for the named widgets.

### UC-UI-02: Show a panel from an extension

- **Actor**: product developer, AI agent.
- **Goal**: a tool, inspector, or settings panel from a WASM extension, in any language.
- **Flow**: publish the panel's widgets on `ui/spec` each frame it should be visible; read the interaction events published back.
- **Success**: the panel floats in the window with the same widgets a native application gets; the extension receives only its own panel's events.

### UC-UI-03: Pick an icon by name

- **Actor**: application developer, product developer, AI agent.
- **Goal**: put a recognisable icon on a button or label without drawing or bundling one.
- **Flow**: name the icon from the bundled set (for example trash, folder-open, settings).
- **Success**: the icon is drawn in the current text colour at the widget's size.

### UC-UI-04: Add a widget bones does not ship

- **Actor**: application developer.
- **Goal**: one widget specific to the product, used next to the standard ones.
- **Flow**: register a widget kind with the engine when composing it, then place that kind in the screen like any other widget.
- **Success**: the widget is drawn and reports events like the standard ones; no bones source was changed.

## Looking native

### UC-UI-05: Follow the desktop's look

- **Actor**: application developer.
- **Goal**: the interface resembles the desktop it runs on: Windows, KDE, GNOME, or COSMIC.
- **Flow**: compose the ui module with the native look (the default).
- **Success**: the interface uses the system font, light or dark mode, the accent colour, and the desktop's spacing and corners, and follows a change of mode or accent while running. Opting out gives egui's own look.

### UC-UI-06: Draw the window's own title bar

- **Actor**: application developer.
- **Goal**: a title bar drawn by the application, as modern desktop applications have, without losing window management.
- **Flow**: put the title bar widget first in the screen's top panel.
- **Success**: the system frame is gone; dragging the bar moves the window, edges resize it, the desktop's snapping works, and the caption buttons minimise, maximise, and close.

## Exclusions

- In-world game menus and HUDs: they use `gfx/*` and the guest `game_ui` module ([ADR-025](../adr/ADR-025-game-ui-is-a-theme-free-guest-toolkit.md)).
- Web technology interfaces: they use web panels.
- Reproducing the platform's real controls: bones draws its own widgets that resemble the platform.
