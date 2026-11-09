# ADR-049: The ui module follows the desktop look

## Problem

egui's default look resembles no desktop. An application built on bones widgets ([ADR-048](ADR-048-one-widget-tree-for-native-modules-and-extensions.md)) cannot style them itself, extensions least of all, so the engine decides how every interface looks.

## Decision

- **Native look by default.** The ui module reads the system font, light or dark mode, and accent colour, and applies the spacing and corners of the desktop it runs on (Windows, KDE, GNOME, COSMIC). It follows mode and accent changes while running. A composition setting turns it off for egui's own look.
- **Resemble, not reproduce.** bones keeps drawing its own widgets; it does not wrap the platform's controls.
- **Lucide icons.** The standard icon set is Lucide, bundled as a font and picked by name. The names are widely known to developers and agents, so an icon can be chosen without looking it up.
- **Custom title bar as a window option.** The platform can open a window without the system frame and hand moving, resizing, and snapping back to the window manager; the title bar itself is a standard widget.

## Rationale

- A product looks at home on its desktop with no code of its own.
- Reading system settings is native I/O, which belongs in a native module, not in extensions.
- Recognisable icon names suit [ADR-044](ADR-044-bones-is-built-for-ai-assisted-product-development.md): an agent writes the right icon without seeing it.

## Rejected alternatives

- **Each application themes itself**: extensions cannot, and native applications would repeat the same detection code.
- **Material Symbols**: the best-known names, but a font of several megabytes and one platform's look everywhere.
- **Phosphor**: a ready egui crate, but less familiar names.
- **Title bar in each application**: the hit-test needs the platform's window, which the platform layer owns.
