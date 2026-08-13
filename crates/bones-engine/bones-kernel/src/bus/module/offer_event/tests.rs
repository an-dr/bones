use std::sync::{Arc, Mutex};

use crate::bus::{Envelope, Handler, Module, ModuleContext};

use super::offer_event;

/// Records how many events it was offered, and claims them or not on
/// command — enough to observe both the return value and the
/// stop-at-the-first-claim behaviour.
struct Filter {
    name: &'static str,
    claims: bool,
    offered: Arc<Mutex<Vec<&'static str>>>,
}

impl Handler for Filter {
    fn handle(&mut self, _envelope: &Envelope) {}
}

impl Module for Filter {
    fn name(&self) -> &str {
        self.name
    }

    fn init(&mut self, _ctx: &mut ModuleContext) -> Result<(), String> {
        Ok(())
    }

    fn filter_event(&mut self, _event: &sdl3::event::Event) -> bool {
        self.offered.lock().unwrap().push(self.name);
        self.claims
    }
}

/// A module that does not override `filter_event` at all, which is the
/// case for audio, game-core, and web.
struct Indifferent;

impl Handler for Indifferent {
    fn handle(&mut self, _envelope: &Envelope) {}
}

impl Module for Indifferent {
    fn name(&self) -> &str {
        "indifferent"
    }

    fn init(&mut self, _ctx: &mut ModuleContext) -> Result<(), String> {
        Ok(())
    }
}

fn boxed(module: impl Module + 'static) -> Arc<Mutex<Box<dyn Module>>> {
    Arc::new(Mutex::new(Box::new(module) as Box<dyn Module>))
}

fn an_event() -> sdl3::event::Event {
    sdl3::event::Event::KeyDown {
        timestamp: 0,
        window_id: 0,
        keycode: Some(sdl3::keyboard::Keycode::A),
        scancode: None,
        keymod: sdl3::keyboard::Mod::empty(),
        repeat: false,
        which: 0,
        raw: 0,
    }
}

#[test]
fn no_modules_claims_nothing() {
    assert!(!offer_event(&[], &an_event()));
}

#[test]
fn a_module_that_does_not_override_the_hook_claims_nothing() {
    let modules = vec![boxed(Indifferent)];

    assert!(!offer_event(&modules, &an_event()));
}

#[test]
fn the_first_claiming_module_stops_the_offer() {
    let offered = Arc::new(Mutex::new(Vec::new()));
    let modules = vec![
        boxed(Filter {
            name: "first",
            claims: true,
            offered: offered.clone(),
        }),
        boxed(Filter {
            name: "second",
            claims: true,
            offered: offered.clone(),
        }),
    ];

    assert!(offer_event(&modules, &an_event()));
    assert_eq!(
        *offered.lock().unwrap(),
        vec!["first"],
        "the second module should never have been offered the event"
    );
}

#[test]
fn an_unclaimed_event_reaches_every_module_and_then_input() {
    let offered = Arc::new(Mutex::new(Vec::new()));
    let modules = vec![
        boxed(Filter {
            name: "first",
            claims: false,
            offered: offered.clone(),
        }),
        boxed(Indifferent),
        boxed(Filter {
            name: "third",
            claims: false,
            offered: offered.clone(),
        }),
    ];

    assert!(
        !offer_event(&modules, &an_event()),
        "nothing claimed it, so platform should still publish input/*"
    );
    assert_eq!(*offered.lock().unwrap(), vec!["first", "third"]);
}

#[test]
fn a_later_module_can_claim_what_an_earlier_one_declined() {
    let offered = Arc::new(Mutex::new(Vec::new()));
    let modules = vec![
        boxed(Filter {
            name: "declines",
            claims: false,
            offered: offered.clone(),
        }),
        boxed(Filter {
            name: "claims",
            claims: true,
            offered: offered.clone(),
        }),
    ];

    assert!(offer_event(&modules, &an_event()));
    assert_eq!(*offered.lock().unwrap(), vec!["declines", "claims"]);
}
