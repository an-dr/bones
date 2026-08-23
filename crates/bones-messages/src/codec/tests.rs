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

#[test]
fn blob_round_trips() {
    let bytes = Writer::new().blob(&[0xde, 0xad, 0xbe, 0xef]).finish();
    let mut r = Reader::new(&bytes);
    assert_eq!(r.read_blob(), Ok(&[0xde, 0xad, 0xbe, 0xef][..]));
    assert_eq!(r.finish(), Ok(()));
}

#[test]
fn two_blobs_in_one_payload_stay_distinct() {
    // Not `read_rest` (which can only ever recover one final field) — the
    // length prefix is what lets a payload carry more than one
    // variable-length byte field.
    let bytes = Writer::new().blob(b"first").blob(b"second").finish();
    let mut r = Reader::new(&bytes);
    assert_eq!(r.read_blob(), Ok(&b"first"[..]));
    assert_eq!(r.read_blob(), Ok(&b"second"[..]));
    assert_eq!(r.finish(), Ok(()));
}

#[test]
fn read_blob_rejects_a_truncated_field() {
    // Declares a 10-byte blob but only supplies 2.
    let mut r = Reader::new(&[10, 0, 0, 0, 1, 2]);
    assert_eq!(r.read_blob(), Err(DecodeError::Truncated));
}

#[test]
fn try_str_matches_str_for_an_ordinary_string() {
    // The fallible writers are an error-handling choice, not a second format:
    // whatever both can encode, they encode identically.
    assert_eq!(
        Writer::new().try_str("main").unwrap().finish(),
        Writer::new().str("main").finish()
    );
}

#[test]
fn try_blob_matches_blob_for_an_ordinary_blob() {
    assert_eq!(
        Writer::new().try_blob(&[1, 2, 3]).unwrap().finish(),
        Writer::new().blob(&[1, 2, 3]).finish()
    );
}

#[test]
fn try_str_reports_a_string_too_long_for_its_prefix() {
    let oversized = "a".repeat(usize::from(u16::MAX) + 1);
    assert_eq!(
        Writer::new().try_str(&oversized).unwrap_err(),
        EncodeError::StringTooLong
    );
}

#[test]
fn try_str_accepts_a_string_exactly_filling_its_prefix() {
    let limit = "a".repeat(usize::from(u16::MAX));
    let bytes = Writer::new().try_str(&limit).unwrap().finish();
    assert_eq!(
        Reader::new(&bytes).read_str().unwrap().len(),
        usize::from(u16::MAX)
    );
}

#[test]
fn try_str_measures_bytes_not_characters() {
    // Just over the limit in bytes while well under it in characters: the
    // check has to agree with the prefix it is protecting.
    let oversized = "é".repeat(usize::from(u16::MAX) / 2 + 1);
    assert!(oversized.chars().count() < usize::from(u16::MAX));
    assert_eq!(
        Writer::new().try_str(&oversized).unwrap_err(),
        EncodeError::StringTooLong
    );
}
