use std::sync::{Arc, Mutex};

use super::Module;

/// Offers one raw platform event to every registered module, topmost
/// first, stopping at the first that claims it (ADR-008).
///
/// Returns whether any module claimed it, which is what the platform's
/// pre-consumption hook uses to decide if the event still becomes an
/// `input/*` message.
///
/// - **Reverse registration order**, because registration order is the
///   layering order and the two run opposite ways: the render and present
///   phases walk it forward so a later module draws *above* an earlier
///   one, and input therefore has to walk it backward so the module that
///   drew last is offered the event first.
/// - Walking it forward would invert ADR-008 outright — the renderer,
///   registered first and drawn underneath everything, would get the first
///   look at a click landing on a ui widget or a web panel above it.
/// - Still no priority field: the rule stays inspectable from the builder
///   call site, since where a module sits in the composition is what
///   decides both where it draws and when it is offered input.
///
/// One lock per module per event, rather than one lock held across the
/// whole poll: the locks are uncontended (dispatch is single-threaded) and
/// holding them across the entire poll would mean a module could not be
/// reached by anything else while events drain.
pub fn offer_event(modules: &[Arc<Mutex<Box<dyn Module>>>], event: &sdl3::event::Event) -> bool {
    modules
        .iter()
        .rev()
        .any(|module| module.lock().unwrap().filter_event(event))
}

#[cfg(test)]
mod tests;
