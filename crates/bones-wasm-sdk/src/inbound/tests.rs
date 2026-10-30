use super::*;
use bones_messages::input::KeyDown;
use bones_messages::EncodeMessage;

#[test]
fn a_core_topic_decodes_to_its_variant() {
    let payload = Tick { dt: 0.5 }.encode();
    assert_eq!(
        Inbound::decode(Tick::TOPIC, &payload),
        Ok(Inbound::Tick(Tick { dt: 0.5 }))
    );
}

#[test]
fn a_borrowed_payload_decodes_without_copying() {
    let payload = KeyDown { key: "Space" }.encode();
    match Inbound::decode(KeyDown::TOPIC, &payload) {
        Ok(Inbound::KeyDown(key)) => assert_eq!(key.key, "Space"),
        other => panic!("expected a key-down, got {other:?}"),
    }
}

#[test]
fn an_unknown_topic_is_other_with_its_payload_intact() {
    assert_eq!(
        Inbound::decode("my-game/score", b"\x07"),
        Ok(Inbound::Other {
            topic: "my-game/score",
            payload: b"\x07",
        })
    );
}

#[test]
fn a_direct_send_has_an_empty_topic_and_is_other() {
    assert!(matches!(
        Inbound::decode("", b"ping"),
        Ok(Inbound::Other { topic: "", .. })
    ));
}

#[test]
fn a_malformed_core_payload_is_an_error_not_other() {
    assert!(Inbound::decode(Tick::TOPIC, b"\x01").is_err());
}

#[test]
fn os_reply_preserves_the_caller_identity() {
    let reply = OsReply {
        requester: "editor",
        request_id: 7,
        accepted: true,
        value: "text",
        error: "",
    };
    let bytes = reply.encode();
    assert_eq!(
        Inbound::decode(OsReply::TOPIC, &bytes),
        Ok(Inbound::OsReply(reply))
    );
}
