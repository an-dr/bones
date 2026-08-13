use super::*;
use crate::{DecodeError, DecodeMessage, EncodeMessage};

#[test]
fn key_messages_round_trip_without_changing_the_wire_format() {
    let down = KeyDown { key: "Space" };
    assert_eq!(down.encode(), b"Space");
    assert_eq!(KeyDown::decode(&down.encode()), Ok(down));

    let up = KeyUp { key: "Escape" };
    assert_eq!(KeyUp::decode(&up.encode()), Ok(up));
}

#[test]
fn mouse_messages_round_trip() {
    let mv = MouseMove {
        x: 12.5,
        y: -3.0,
        dx: 1.0,
        dy: 0.0,
    };
    assert_eq!(MouseMove::decode(&mv.encode()), Ok(mv));

    let down = MouseDown {
        button: 1,
        x: 10.0,
        y: 20.0,
    };
    assert_eq!(MouseDown::decode(&down.encode()), Ok(down));

    let up = MouseUp {
        button: 3,
        x: 10.0,
        y: 20.0,
    };
    assert_eq!(MouseUp::decode(&up.encode()), Ok(up));

    let wheel = MouseWheel { x: 0.0, y: 1.0 };
    assert_eq!(MouseWheel::decode(&wheel.encode()), Ok(wheel));
}

#[test]
fn mouse_messages_reject_wrong_byte_counts() {
    assert_eq!(MouseMove::decode(&[0; 15]), Err(DecodeError::Truncated));
    assert_eq!(MouseDown::decode(&[0; 8]), Err(DecodeError::Truncated));
    assert_eq!(MouseWheel::decode(&[0; 7]), Err(DecodeError::Truncated));
}

#[test]
fn gamepad_messages_round_trip() {
    let connected = GamepadConnected { id: 7 };
    assert_eq!(GamepadConnected::decode(&connected.encode()), Ok(connected));

    let disconnected = GamepadDisconnected { id: 7 };
    assert_eq!(
        GamepadDisconnected::decode(&disconnected.encode()),
        Ok(disconnected)
    );

    let axis = GamepadAxis {
        id: 1,
        axis: "LeftX",
        value: -0.5,
    };
    assert_eq!(GamepadAxis::decode(&axis.encode()), Ok(axis));

    let down = GamepadButtonDown {
        id: 1,
        button: "South",
    };
    assert_eq!(GamepadButtonDown::decode(&down.encode()), Ok(down));

    let up = GamepadButtonUp {
        id: 1,
        button: "South",
    };
    assert_eq!(GamepadButtonUp::decode(&up.encode()), Ok(up));
}
