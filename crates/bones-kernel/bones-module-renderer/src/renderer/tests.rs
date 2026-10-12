use super::*;

#[test]
fn loaded_and_reloaded_extensions_receive_the_logical_canvas() {
    assert!(announces_logical_canvas(Event::Loaded));
    assert!(announces_logical_canvas(Event::Reloaded));
    assert!(!announces_logical_canvas(Event::Faulted));
    assert!(!announces_logical_canvas(Event::Reloading));
    assert!(!announces_logical_canvas(Event::Stopped));
}
