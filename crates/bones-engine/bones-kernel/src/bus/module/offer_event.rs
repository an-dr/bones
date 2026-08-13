use std::sync::{Arc, Mutex};

use super::Module;

/// Offers one raw platform event to every registered module in
/// registration order, stopping at the first that claims it (ADR-008).
///
/// Returns whether any module claimed it, which is what the platform's
/// pre-consumption hook uses to decide if the event still becomes an
/// `input/*` message.
///
/// Registration order *is* the layering order: a module registered later
/// sits above one registered earlier only in the sense of drawing, so
/// input is offered in the order modules were composed, and the first
/// claim wins. That makes the rule inspectable from the builder call site
/// rather than hidden in a priority field nothing sets.
///
/// One lock per module per event, rather than one lock held across the
/// whole poll: the locks are uncontended (dispatch is single-threaded) and
/// holding them across the entire poll would mean a module could not be
/// reached by anything else while events drain.
pub fn offer_event(
    modules: &[Arc<Mutex<Box<dyn Module>>>],
    event: &sdl3::event::Event,
) -> bool {
    modules
        .iter()
        .any(|module| module.lock().unwrap().filter_event(event))
}

#[cfg(test)]
mod tests;
