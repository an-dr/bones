# game-ui

Reusable game-rendered UI mechanics for bones WASM guests. It provides a
configurable logical canvas, vertical button layout, wrapped keyboard
selection, physical-window mouse hit-testing, and owned rectangle/text
commands that publish through a caller-supplied host callback.

The crate deliberately has no theme, screen navigation, fixed labels, or WIT
host dependency. Games choose their own colors and flow while reusing the same
geometry and input behavior. It emits ordinary `bones-messages` `gfx/*`
payloads, so no native module is required.
