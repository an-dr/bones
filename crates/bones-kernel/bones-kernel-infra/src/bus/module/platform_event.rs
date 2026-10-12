/// The raw platform event [`Module::filter_event`](super::Module::filter_event)
/// is offered, before anything translates it onto `input/*`.
///
/// - An alias for SDL's own event, not a wrapper: ADR-031 chose the raw
///   type deliberately, because the neutral `input/*` vocabulary carries
///   neither text input nor modifier state and a translation layer would
///   silently drop what an egui-style layer needs.
/// - It exists as a *name* so a module author can write the hook's
///   signature against the engine's public surface rather than adding
///   `sdl3` at a matching version to their own manifest. The facade
///   re-exports this; it does not re-export `sdl3`.
/// - Feature-gated with `platform` alongside the hook itself, since a
///   headless build has no event source to name.
pub type PlatformEvent = sdl3::event::Event;
