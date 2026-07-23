use super::*;
use crate::{DecodeError, EncodeMessage, Message};

#[test]
fn non_gfx_topics_are_ignored() {
    assert_eq!(Command::decode("input/key-down", b"whatever"), Ok(None));
}

#[test]
fn every_command_round_trips() {
    let clear = Clear {
        r: 10,
        g: 20,
        b: 30,
        a: 255,
    };
    assert_eq!(
        Command::decode(Clear::TOPIC, &clear.encode()),
        Ok(Some(Command::Clear(clear)))
    );

    let load = LoadSprite {
        id: 7,
        png_bytes: b"not-really-a-png",
    };
    let encoded = load.encode();
    assert_eq!(
        Command::decode(LoadSprite::TOPIC, &encoded),
        Ok(Some(Command::LoadSprite(load)))
    );

    let draw = DrawSprite {
        id: 7,
        dst_x: 100,
        dst_y: 200,
        dst_w: 96,
        dst_h: 96,
        src_x: 0,
        src_y: 0,
        src_w: 64,
        src_h: 64,
        layer: 3,
        angle: 45.0,
        flip_h: true,
        flip_v: false,
        tint: (255, 128, 64, 255),
    };
    assert_eq!(
        Command::decode(DrawSprite::TOPIC, &draw.encode()),
        Ok(Some(Command::DrawSprite(draw)))
    );

    let camera = SetCamera {
        x: 12.5,
        y: -4.0,
        zoom: 2.0,
    };
    assert_eq!(
        Command::decode(SetCamera::TOPIC, &camera.encode()),
        Ok(Some(Command::SetCamera(camera)))
    );

    let rect = DrawRect {
        x: 10,
        y: 20,
        w: 30,
        h: 40,
        filled: true,
        color: (255, 0, 0, 255),
        layer: 2,
        screen_space: false,
    };
    assert_eq!(
        Command::decode(DrawRect::TOPIC, &rect.encode()),
        Ok(Some(Command::DrawRect(rect)))
    );

    let line = DrawLine {
        x1: 0,
        y1: 0,
        x2: 50,
        y2: 60,
        color: (0, 255, 0, 255),
        layer: 1,
    };
    assert_eq!(
        Command::decode(DrawLine::TOPIC, &line.encode()),
        Ok(Some(Command::DrawLine(line)))
    );

    let circle = DrawCircle {
        x: 5,
        y: 5,
        radius: 15,
        filled: false,
        color: (0, 0, 255, 255),
        layer: 4,
    };
    assert_eq!(
        Command::decode(DrawCircle::TOPIC, &circle.encode()),
        Ok(Some(Command::DrawCircle(circle)))
    );

    let text = DrawText {
        text: "hp: 30/30",
        x: 8,
        y: 8,
        size: 16,
        color: (255, 255, 255, 255),
        layer: 9,
        screen_space: false,
    };
    assert_eq!(
        Command::decode(DrawText::TOPIC, &text.encode()),
        Ok(Some(Command::DrawText(text)))
    );

    let triangle = DrawTriangle {
        x1: 10,
        y1: 0,
        x2: 0,
        y2: 20,
        x3: 20,
        y3: 20,
        filled: true,
        color: (128, 0, 255, 255),
        layer: 5,
    };
    assert_eq!(
        Command::decode(DrawTriangle::TOPIC, &triangle.encode()),
        Ok(Some(Command::DrawTriangle(triangle)))
    );
}

#[test]
fn screen_space_draws_round_trip() {
    let rect = DrawRect {
        x: 10,
        y: 20,
        w: 30,
        h: 40,
        filled: true,
        color: (255, 0, 0, 255),
        layer: 2,
        screen_space: true,
    };
    assert_eq!(
        Command::decode(DrawRect::TOPIC, &rect.encode()),
        Ok(Some(Command::DrawRect(rect)))
    );

    let text = DrawText {
        text: "score: 0",
        x: 8,
        y: 8,
        size: 16,
        color: (255, 255, 255, 255),
        layer: 9,
        screen_space: true,
    };
    assert_eq!(
        Command::decode(DrawText::TOPIC, &text.encode()),
        Ok(Some(Command::DrawText(text)))
    );
}

#[test]
fn fixed_shape_commands_reject_wrong_byte_counts() {
    assert_eq!(
        Command::decode(Clear::TOPIC, &[1, 2, 3]),
        Err(DecodeError::Truncated)
    );
    assert_eq!(
        Command::decode(DrawSprite::TOPIC, &[0; 46]),
        Err(DecodeError::Truncated)
    );
}
