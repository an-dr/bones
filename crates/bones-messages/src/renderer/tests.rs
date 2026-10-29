use super::*;

#[test]
fn decode_reads_back_what_encode_wrote() {
    let changed = DisplayChanged {
        width: 1280,
        height: 720,
    };
    assert_eq!(DisplayChanged::decode(&changed.encode()), Ok(changed));

    let logical = LogicalCanvas {
        width: 800,
        height: 600,
    };
    assert_eq!(LogicalCanvas::decode(&logical.encode()), Ok(logical));
}

#[test]
fn decode_rejects_the_wrong_byte_count() {
    assert_eq!(
        DisplayChanged::decode(&[1, 2, 3]),
        Err(DecodeError::Truncated)
    );
    assert_eq!(
        DisplayChanged::decode(&[0; 9]),
        Err(DecodeError::TrailingBytes)
    );
    assert_eq!(
        LogicalCanvas::decode(&[1, 2, 3]),
        Err(DecodeError::Truncated)
    );
    assert_eq!(
        LogicalCanvas::decode(&[0; 9]),
        Err(DecodeError::TrailingBytes)
    );
}

#[test]
fn a_capture_and_its_reply_round_trip() {
    let capture = Capture {
        request_id: 9,
        scope: Scope::Window,
    };
    assert_eq!(Capture::decode(&capture.encode()), Ok(capture));

    let captured = Captured {
        request_id: 9,
        scope: Scope::Canvas,
        width: 4,
        height: 3,
        png: b"\x89PNG",
    };
    assert_eq!(Captured::decode(&captured.encode()), Ok(captured));
}

#[test]
fn a_failed_capture_carries_no_image() {
    let failed = Captured {
        request_id: 1,
        scope: Scope::Window,
        width: 0,
        height: 0,
        png: &[],
    };
    assert_eq!(Captured::decode(&failed.encode()), Ok(failed));
}

#[test]
fn an_unknown_scope_is_an_invalid_tag() {
    assert_eq!(
        Capture::decode(&[1, 0, 0, 0, 2]),
        Err(DecodeError::InvalidTag {
            message: "capture scope",
            tag: 2
        })
    );
}
