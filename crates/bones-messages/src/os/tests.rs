use super::{Action, Request, Result};
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message};

#[test]
fn request_round_trips_every_action() {
    for action in [
        Action::ReadClipboard,
        Action::WriteClipboard,
        Action::OpenUrl,
        Action::PickFile,
        Action::PickFolder,
        Action::OpenDirectory,
        Action::FetchUrl,
    ] {
        let request = Request {
            request_id: 7,
            action,
            value: "value",
        };
        assert_eq!(Request::decode(&request.encode()).unwrap(), request);
    }
}

#[test]
fn request_frames_id_then_tag_then_value() {
    let bytes = Request {
        request_id: 1,
        action: Action::OpenUrl,
        value: "hi",
    }
    .encode();
    assert_eq!(bytes, [1, 0, 0, 0, 2, 2, 0, b'h', b'i']);
}

#[test]
fn request_tags_are_stable() {
    // These are the wire format. Changing one silently repoints every guest
    // already built against it, so they are asserted rather than derived.
    let tag = |action| {
        Request {
            request_id: 0,
            action,
            value: "",
        }
        .encode()[4]
    };
    assert_eq!(tag(Action::ReadClipboard), 0);
    assert_eq!(tag(Action::WriteClipboard), 1);
    assert_eq!(tag(Action::OpenUrl), 2);
    assert_eq!(tag(Action::PickFile), 3);
    assert_eq!(tag(Action::PickFolder), 4);
    assert_eq!(tag(Action::OpenDirectory), 5);
    assert_eq!(tag(Action::FetchUrl), 6);
}

#[test]
fn request_refuses_an_unknown_action() {
    assert_eq!(
        Request::decode(&[1, 0, 0, 0, 7, 0, 0]),
        Err(DecodeError::InvalidTag {
            message: "os action",
            tag: 7
        })
    );
}

#[test]
fn request_refuses_trailing_bytes() {
    let mut bytes = Request {
        request_id: 1,
        action: Action::PickFile,
        value: "",
    }
    .encode();
    bytes.push(0);
    assert_eq!(Request::decode(&bytes), Err(DecodeError::TrailingBytes));
}

#[test]
fn result_round_trips_both_outcomes() {
    for result in [
        Result {
            request_id: 3,
            accepted: true,
            value: "C:/chosen.txt",
            error: "",
        },
        Result {
            request_id: 4,
            accepted: false,
            value: "",
            error: "unsafe URL",
        },
    ] {
        assert_eq!(Result::decode(&result.encode()).unwrap(), result);
    }
}

#[test]
fn a_refused_result_is_not_an_error() {
    // A cancelled dialog: nothing was chosen, and nothing went wrong. The
    // two are distinguishable only because `accepted` exists.
    let cancelled = Result {
        request_id: 5,
        accepted: false,
        value: "",
        error: "",
    };
    let bytes = cancelled.encode();
    let decoded = Result::decode(&bytes).unwrap();
    assert!(!decoded.accepted);
    assert!(decoded.error.is_empty());
}

#[test]
fn result_refuses_a_non_boolean_accepted() {
    assert_eq!(
        Result::decode(&[1, 0, 0, 0, 2, 0, 0, 0, 0]),
        Err(DecodeError::InvalidTag {
            message: "os result accepted",
            tag: 2
        })
    );
}

#[test]
fn topics_are_the_documented_names() {
    assert_eq!(Request::TOPIC, "os/request");
    assert_eq!(Result::TOPIC, "os/result");
}
