use super::{Action, Outcome, Request, Result};
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message};

#[test]
fn request_round_trips_every_action() {
    for action in [Action::Register, Action::Unregister, Action::Status] {
        let request = Request {
            request_id: 9,
            action,
        };
        assert_eq!(Request::decode(&request.encode()).unwrap(), request);
    }
}

#[test]
fn request_tags_are_stable() {
    let tag = |action| {
        Request {
            request_id: 0,
            action,
        }
        .encode()[4]
    };
    assert_eq!(tag(Action::Register), 0);
    assert_eq!(tag(Action::Unregister), 1);
    assert_eq!(tag(Action::Status), 2);
}

#[test]
fn request_refuses_an_unknown_action() {
    assert_eq!(
        Request::decode(&[1, 0, 0, 0, 3]),
        Err(DecodeError::InvalidTag {
            message: "desktop action",
            tag: 3
        })
    );
}

#[test]
fn result_round_trips_both_outcomes() {
    for outcome in [
        Outcome::Registered(true),
        Outcome::Registered(false),
        Outcome::Failed("no home directory"),
    ] {
        let result = Result {
            request_id: 4,
            outcome,
        };
        assert_eq!(Result::decode(&result.encode()).unwrap(), result);
    }
}

#[test]
fn result_frames_id_then_tag_then_fields() {
    let registered = Result {
        request_id: 1,
        outcome: Outcome::Registered(true),
    }
    .encode();
    assert_eq!(registered, [1, 0, 0, 0, 0, 1]);
    let failed = Result {
        request_id: 1,
        outcome: Outcome::Failed("no"),
    }
    .encode();
    assert_eq!(failed, [1, 0, 0, 0, 1, b'n', b'o']);
}

#[test]
fn result_refuses_an_unknown_outcome_and_trailing_bytes() {
    assert_eq!(
        Result::decode(&[1, 0, 0, 0, 2]),
        Err(DecodeError::InvalidTag {
            message: "desktop result",
            tag: 2
        })
    );
    assert_eq!(
        Result::decode(&[1, 0, 0, 0, 0, 1, 0]),
        Err(DecodeError::TrailingBytes)
    );
}

#[test]
fn topics_are_the_documented_names() {
    assert_eq!(Request::TOPIC, "desktop/request");
    assert_eq!(Result::TOPIC, "desktop/result");
}
