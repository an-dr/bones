use super::*;
use bones_messages::DecodeMessage;
use std::sync::{Arc, Mutex};

#[test]
fn parse_recovers_what_publish_encoded() {
    let bus = Bus::new();
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    let ep = bus.register("watcher", move |e: &Envelope| {
        sink.lock().unwrap().push(e.clone())
    });
    ep.subscribe(LifecycleEvent::TOPIC);

    publish(&bus, "engine", "level", Event::Reloaded);
    bus.dispatch();

    let got = received.lock().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].sender, "engine");
    assert_eq!(
        LifecycleEvent::decode(&got[0].payload),
        Ok(LifecycleEvent {
            event: Event::Reloaded,
            extension: "level"
        })
    );
}
