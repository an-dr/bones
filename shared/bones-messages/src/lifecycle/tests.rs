use super::*;

#[test]
fn every_event_round_trips() {
    for event in [
        Event::Loaded,
        Event::Faulted,
        Event::Reloading,
        Event::Reloaded,
        Event::Stopped,
    ] {
        let message = LifecycleEvent {
            event,
            extension: "level",
        };
        assert_eq!(LifecycleEvent::decode(&message.encode()), Ok(message));
    }
}

#[test]
fn invalid_event_and_name_are_structured_errors() {
    assert_eq!(
        LifecycleEvent::decode(&[255, b'x']),
        Err(DecodeError::InvalidTag {
            message: "lifecycle event",
            tag: 255
        })
    );
    assert_eq!(
        LifecycleEvent::decode(&[0, 0xff]),
        Err(DecodeError::InvalidUtf8)
    );
}
