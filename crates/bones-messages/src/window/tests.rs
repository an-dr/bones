use super::*;
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message};

#[test]
fn close_requested_uses_the_window_topic_and_empty_payload() {
    assert_eq!(CloseRequested::TOPIC, "window/close-requested");
    assert!(CloseRequested.encode().is_empty());
    assert_eq!(CloseRequested::decode(&[]), Ok(CloseRequested));
}

#[test]
fn close_requested_rejects_unexpected_payload_bytes() {
    assert_eq!(
        CloseRequested::decode(&[0]),
        Err(DecodeError::TrailingBytes)
    );
}
