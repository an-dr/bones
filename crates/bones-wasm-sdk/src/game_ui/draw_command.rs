use bones_messages::gfx::{DrawRect, DrawText, TextAlign};
use bones_messages::{EncodeMessage, Message};

use crate::game_ui::Rect;

/// Owned game-UI draw command that emits an ordinary `gfx/*` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawCommand {
    /// A screen-space rectangle, published as `gfx/draw-rect`.
    Rectangle {
        /// Where to draw, in logical pixels.
        bounds: Rect,
        /// Fills the rectangle instead of outlining it.
        filled: bool,
        /// RGBA color.
        color: (u8, u8, u8, u8),
        /// Composite order; higher layers draw above lower ones.
        layer: u8,
    },
    /// Screen-space text, published as `gfx/draw-text`.
    Text {
        /// The text to draw.
        text: String,
        /// Anchor x, in logical pixels; `align` decides what it anchors.
        x: i32,
        /// Anchor y, in logical pixels.
        y: i32,
        /// Font size in logical pixels.
        size: u16,
        /// RGBA color.
        color: (u8, u8, u8, u8),
        /// Composite order; higher layers draw above lower ones.
        layer: u8,
        /// Whether `x` marks the text's left edge, center, or right edge.
        align: TextAlign,
    },
}

impl DrawCommand {
    /// A rectangle command.
    pub fn rectangle(bounds: Rect, filled: bool, color: (u8, u8, u8, u8), layer: u8) -> Self {
        Self::Rectangle {
            bounds,
            filled,
            color,
            layer,
        }
    }

    /// A text command anchored by `align`.
    pub fn text_aligned(
        text: impl Into<String>,
        x: i32,
        y: i32,
        size: u16,
        color: (u8, u8, u8, u8),
        layer: u8,
        align: TextAlign,
    ) -> Self {
        Self::Text {
            text: text.into(),
            x,
            y,
            size,
            color,
            layer,
            align,
        }
    }

    /// A left-aligned text command.
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
            align: TextAlign::Left,
        }
    }

    /// Encodes the command and hands its topic and payload to `publish`,
    /// which is usually the host's `publish` import.
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
                align,
            } => {
                let message = DrawText {
                    text,
                    x: *x,
                    y: *y,
                    size: *size,
                    color: *color,
                    layer: *layer,
                    screen_space: true,
                    align: *align,
                };
                publish(DrawText::TOPIC, &message.encode());
            }
        }
    }
}
