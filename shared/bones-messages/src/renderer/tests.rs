use super::*;

#[test]
fn decode_reads_back_what_encode_wrote() {
    let changed = DisplayChanged {
        width: 1280,
        height: 720,
    };
    assert_eq!(DisplayChanged::decode(&changed.encode()), Ok(changed));
}

#[test]
fn decode_rejects_the_wrong_byte_count() {
    assert_eq!(DisplayChanged::decode(&[1, 2, 3]), Err(DecodeError::Truncated));
    assert_eq!(
        DisplayChanged::decode(&[0; 9]),
        Err(DecodeError::TrailingBytes)
    );
}
