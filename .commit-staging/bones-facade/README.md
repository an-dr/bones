# bones

The facade crate: the one name an embedder depends on (design/modules.md).
Re-exports `Engine`, `BuiltEngine`, and `Supervisor` from `bones-runner`.

`app` (the shipped engine executable) is built on this same facade, not on
`bones-runner` directly — so an embedder using `bones` has no access the
shipped app lacks (ADR-016, structure.md's "no privileged access" rule).
