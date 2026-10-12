use super::Launched;
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message};

#[test]
fn launched_round_trips() {
    let message = Launched {
        args: vec!["--x", "y", r"C:\path with spaces"],
        cwd: r"C:\work",
    };
    assert_eq!(Launched::decode(&message.encode()).unwrap(), message);
}

#[test]
fn launched_frames_cwd_then_count_then_args() {
    let bytes = Launched {
        args: vec!["a", "bc"],
        cwd: "d",
    }
    .encode();
    assert_eq!(bytes, [1, 0, b'd', 2, 0, 1, 0, b'a', 2, 0, b'b', b'c']);
}

#[test]
fn launched_with_no_arguments_is_a_count_of_zero() {
    let bytes = Launched {
        args: Vec::new(),
        cwd: "",
    }
    .encode();
    assert_eq!(bytes, [0, 0, 0, 0]);
}

#[test]
fn launched_refuses_trailing_bytes() {
    let mut bytes = Launched {
        args: vec!["a"],
        cwd: "d",
    }
    .encode();
    bytes.push(0);
    assert_eq!(Launched::decode(&bytes), Err(DecodeError::TrailingBytes));
}

#[test]
fn launched_reports_an_argument_too_long_to_frame_instead_of_panicking() {
    let long = "x".repeat(usize::from(u16::MAX) + 1);
    let message = Launched {
        args: vec![&long],
        cwd: "",
    };
    assert_eq!(message.try_encode(), None);
}

#[test]
fn topic_is_the_documented_name() {
    assert_eq!(Launched::TOPIC, "instance/launched");
}
