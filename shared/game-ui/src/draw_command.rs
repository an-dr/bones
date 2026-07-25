use bones_messages::gfx::{DrawRect, DrawText};
use bones_messages::{EncodeMessage, Message};

use crate::Rect;

/// Owned game-UI draw command that emits an ordinary `gfx/*` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawCommand {
    Rectangle {
        bounds: Rect,
        filled: bool,
        color: (u8, u8, u8, u8),
        layer: u8,
    },
    Text {
        text: String,
        x: i32,
        y: i32,
        size: u16,
        color: (u8, u8, u8, u8),
        layer: u8,
    },
}

impl DrawCommand {
    pub fn rectangle(bounds: Rect, filled: bool, color: (u8, u8, u8, u8), layer: u8) -> Self {
        Self::Rectangle {
            bounds,
            filled,
            color,
            layer,
        }
    }

    pub fn text(
        text: impl Into<String>,
        x: i32,
        y: i32,
        size: u16,
        color: (u8, u8, u8, u8),
        layer: u8,
    ) -> Self {
        Self::Text {
            text: text.into(),
            x,
            y,
            size,
            color,
            layer,
        }
    }

    pub fn publish_with(&self, mut publish: impl FnMut(&str, &[u8])) {
        match self {
            Self::Rectangle {
                bounds,
                filled,
                color,
                layer,
            } => {
                let message = DrawRect {
                    x: bounds.x,
                    y: bounds.y,
                    w: bounds.width,
                    h: bounds.height,
                    filled: *filled,
                    color: *color,
                    layer: *layer,
                    screen_space: true,
                };
                publish(DrawRect::TOPIC, &message.encode());
            }
            Self::Text {
                text,
                x,
                y,
                size,
                color,
                layer,
            } => {
                let message = DrawText {
                    text,
                    x: *x,
                    y: *y,
                    size: *size,
                    color: *color,
                    layer: *layer,
                    screen_space: true,
                };
                publish(DrawText::TOPIC, &message.encode());
            }
        }
    }
}
