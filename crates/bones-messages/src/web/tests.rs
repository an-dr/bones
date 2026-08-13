use super::*;
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Writer};

#[test]
fn every_direct_command_round_trips() {
    let commands = [
        Command::Open(OpenPanel {
            panel: "dashboard",
            source: PanelSource::Html("<h1>Status</h1>"),
        }),
        Command::Open(OpenPanel {
            panel: "remote",
            source: PanelSource::Url("https://example.com/status"),
        }),
        Command::Close(ClosePanel { panel: "dashboard" }),
        Command::Navigate(Navigate {
            panel: "dashboard",
            url: "https://example.com/next",
        }),
        Command::SendJson(SendJson {
            panel: "dashboard",
            json: r#"{"online":true}"#,
        }),
    ];

    for command in commands {
        assert_eq!(Command::decode(&command.encode()), Ok(command));
    }
}

#[test]
fn malformed_commands_are_rejected_without_allocating() {
    assert_eq!(Command::decode(&[]), Err(DecodeError::Truncated));
    assert_eq!(
        Command::decode(&[255]),
        Err(DecodeError::InvalidTag {
            message: "web command",
            tag: 255,
        })
    );
    let unknown_source = Writer::new().u8(0).str("panel").u8(9).finish();
    assert_eq!(
        Command::decode(&unknown_source),
        Err(DecodeError::InvalidTag {
            message: "web panel source",
            tag: 9,
        })
    );
    let truncated_panel = Writer::new().u8(2).u16(4).bytes(b"x").finish();
    assert_eq!(
        Command::decode(&truncated_panel),
        Err(DecodeError::Truncated)
    );
    assert_eq!(Command::decode(&[1, 0xff]), Err(DecodeError::InvalidUtf8));
}

#[test]
fn lifecycle_events_round_trip_on_stable_topics() {
    let opened = PanelOpened {
        owner: "dashboard",
        panel: "status",
    };
    let closed = PanelClosed {
        owner: "dashboard",
        panel: "status",
    };
    let failed = PanelFailed {
        owner: "dashboard",
        panel: "status",
        reason: "backend unavailable",
    };

    assert_eq!(PanelOpened::TOPIC, "web/panel-opened");
    assert_eq!(PanelClosed::TOPIC, "web/panel-closed");
    assert_eq!(PanelFailed::TOPIC, "web/panel-failed");
    assert_eq!(PanelOpened::decode(&opened.encode()), Ok(opened));
    assert_eq!(PanelClosed::decode(&closed.encode()), Ok(closed));
    assert_eq!(PanelFailed::decode(&failed.encode()), Ok(failed));
}

#[test]
fn page_json_is_opaque_and_borrowed() {
    let message = PageMessage {
        owner: "dashboard",
        panel: "status",
        json: "not validated as json",
    };

    assert_eq!(PageMessage::TOPIC, "web/page-message");
    assert_eq!(PageMessage::decode(&message.encode()), Ok(message));
}

#[test]
fn event_decoders_reject_invalid_utf8_and_truncated_fields() {
    assert_eq!(
        PanelOpened::decode(&[1, 0, b'o', 0xff]),
        Err(DecodeError::InvalidUtf8)
    );
    assert_eq!(
        PanelFailed::decode(&[4, 0, b'x']),
        Err(DecodeError::Truncated)
    );
    assert_eq!(
        PageMessage::decode(&[1, 0, b'o', 1, 0, b'p', 0xff]),
        Err(DecodeError::InvalidUtf8)
    );
}
