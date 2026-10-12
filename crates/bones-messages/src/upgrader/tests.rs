use super::{Action, Outcome, Request, Result};
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message};

#[test]
fn request_round_trips_every_action() {
    for action in [
        Action::Check,
        Action::Stage,
        Action::Install,
        Action::LastApplied,
    ] {
        let request = Request {
            request_id: 2,
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
    assert_eq!(tag(Action::Check), 0);
    assert_eq!(tag(Action::Stage), 1);
    assert_eq!(tag(Action::Install), 2);
    assert_eq!(tag(Action::LastApplied), 3);
}

#[test]
fn request_refuses_an_unknown_action() {
    assert_eq!(
        Request::decode(&[1, 0, 0, 0, 4]),
        Err(DecodeError::InvalidTag {
            message: "upgrader action",
            tag: 4
        })
    );
}

#[test]
fn result_round_trips_every_outcome() {
    for outcome in [
        Outcome::Available { version: "1.2.0" },
        Outcome::UpToDate,
        Outcome::Staged { version: "1.2.0" },
        Outcome::Installed { fresh: true },
        Outcome::Installed { fresh: false },
        Outcome::LastApplied {
            previous: Some("1.1.0"),
            current: "1.2.0",
        },
        Outcome::LastApplied {
            previous: None,
            current: "1.2.0",
        },
        Outcome::Failed { reason: "offline" },
    ] {
        let result = Result {
            request_id: 6,
            outcome,
        };
        assert_eq!(Result::decode(&result.encode()).unwrap(), result);
    }
}

#[test]
fn result_tags_are_stable() {
    let tag = |outcome| {
        Result {
            request_id: 0,
            outcome,
        }
        .encode()[4]
    };
    assert_eq!(tag(Outcome::Available { version: "" }), 0);
    assert_eq!(tag(Outcome::UpToDate), 1);
    assert_eq!(tag(Outcome::Staged { version: "" }), 2);
    assert_eq!(tag(Outcome::Installed { fresh: false }), 3);
    assert_eq!(
        tag(Outcome::LastApplied {
            previous: None,
            current: ""
        }),
        4
    );
    assert_eq!(tag(Outcome::Failed { reason: "" }), 5);
}

#[test]
fn last_applied_frames_presence_then_previous_then_current() {
    let bytes = Result {
        request_id: 1,
        outcome: Outcome::LastApplied {
            previous: Some("1"),
            current: "2",
        },
    }
    .encode();
    assert_eq!(bytes, [1, 0, 0, 0, 4, 1, 1, 0, b'1', b'2']);
}

#[test]
fn result_refuses_an_unknown_outcome_and_trailing_bytes() {
    assert_eq!(
        Result::decode(&[1, 0, 0, 0, 6]),
        Err(DecodeError::InvalidTag {
            message: "upgrader result",
            tag: 6
        })
    );
    assert_eq!(
        Result::decode(&[1, 0, 0, 0, 1, 0]),
        Err(DecodeError::TrailingBytes)
    );
}

#[test]
fn topics_are_the_documented_names() {
    assert_eq!(Request::TOPIC, "upgrader/request");
    assert_eq!(Result::TOPIC, "upgrader/result");
}
