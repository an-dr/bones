use super::*;

#[test]
fn decode_reads_back_what_encode_wrote() {
    let tick = Tick { dt: 1.0 / 60.0 };
    assert_eq!(Tick::decode(&tick.encode()), Ok(tick));
}

#[test]
fn decode_rejects_the_wrong_byte_count() {
    assert_eq!(Tick::decode(&[1, 2, 3]), Err(DecodeError::Truncated));
    assert_eq!(Tick::decode(&[0; 5]), Err(DecodeError::TrailingBytes));
}
