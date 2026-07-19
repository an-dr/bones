use super::*;

#[test]
fn writer_produces_little_endian_bytes_in_call_order() {
    let bytes = Writer::new().u8(1).u32(2).i32(-1).bytes(b"xy").finish();
    assert_eq!(bytes, [1, 2, 0, 0, 0, 255, 255, 255, 255, b'x', b'y']);
}

#[test]
fn reader_reads_back_what_writer_wrote() {
    let bytes = Writer::new().u32(7).i32(-100).finish();
    let mut r = Reader::new(&bytes);
    assert_eq!(r.read_u32(), Ok(7));
    assert_eq!(r.read_i32(), Ok(-100));
    assert_eq!(r.finish(), Ok(()));
}

#[test]
fn reader_rejects_a_truncated_field() {
    let mut r = Reader::new(&[0, 0]);
    assert_eq!(r.read_u32(), Err(DecodeError::Truncated));
}

#[test]
fn finish_rejects_trailing_bytes() {
    let mut r = Reader::new(&[1, 2, 3, 4, 5]);
    r.read_u32().unwrap();
    assert_eq!(r.finish(), Err(DecodeError::TrailingBytes));
}

#[test]
fn read_rest_takes_every_remaining_byte() {
    let mut r = Reader::new(&[1, 2, 3, 4, 5]);
    r.read_u8().unwrap();
    assert_eq!(r.read_rest(), &[2, 3, 4, 5]);
}

#[test]
fn string_reader_borrows_valid_utf8_and_rejects_invalid_utf8() {
    let mut valid = Reader::new(b"level");
    assert_eq!(valid.read_str_rest(), Ok("level"));

    let mut invalid = Reader::new(&[0xff]);
    assert_eq!(invalid.read_str_rest(), Err(DecodeError::InvalidUtf8));
}
