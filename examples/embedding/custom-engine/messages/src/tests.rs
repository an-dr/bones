use super::*;

/// A custom vocabulary is a wire contract like any other: once an extension is
/// built against it, changing the bytes breaks that extension. These are the
/// same round-trip tests `bones-messages` keeps for its own topics, and they
/// exist here for the same reason.
#[test]
fn every_fact_round_trips() {
    for fact in [
        Fact::Hostname,
        Fact::WorkingDirectory,
        Fact::EnvironmentVariable("PATH"),
        // The empty name is worth covering: it is the boundary where a
        // `read_str_rest` of nothing has to succeed rather than truncate.
        Fact::EnvironmentVariable(""),
    ] {
        let encoded = FactsRequest { fact }.encode();
        assert_eq!(FactsRequest::decode(&encoded), Ok(FactsRequest { fact }));
    }
}

#[test]
fn a_reply_round_trips_including_the_empty_one() {
    for value in ["bones-dev", ""] {
        let encoded = FactsReply { value }.encode();
        assert_eq!(FactsReply::decode(&encoded), Ok(FactsReply { value }));
    }
}

#[test]
fn an_unknown_tag_is_rejected_rather_than_guessed() {
    // What a guest built against a newer vocabulary would send. The payload
    // does not carry the length of its own variant, so a decoder that does not
    // know the tag cannot skip it (`wit/wire-format.md`).
    assert_eq!(
        FactsRequest::decode(&[99]),
        Err(DecodeError::InvalidTag {
            message: "host fact",
            tag: 99
        })
    );
}

#[test]
fn an_empty_payload_is_truncated_not_a_default() {
    assert_eq!(FactsRequest::decode(&[]), Err(DecodeError::Truncated));
}
