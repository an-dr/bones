use bones_messages::gfx::{DrawCircle, DrawLine, DrawRect, DrawSprite};

/// A `gfx/*` draw command that goes through the retained-batch/layer
/// pipeline (as opposed to `Clear`/`LoadSprite`/`SetCamera`, which apply
/// immediately as retained singletons — see `Inner::record`).
// No `Copy`: `Text` owns a `String` (the source `DrawText<'a>`'s `&str`
// borrows the envelope payload, which doesn't outlive one `record` call).
#[derive(Clone)]
pub(crate) enum RetainedDraw {
    Sprite(DrawSprite),
    Rect(DrawRect),
    Line(DrawLine),
    Circle(DrawCircle),
    Text {
        text: String,
        x: i32,
        y: i32,
        size: u16,
        color: (u8, u8, u8, u8),
        layer: u8,
    },
}

impl RetainedDraw {
    pub(crate) fn layer(&self) -> u8 {
        match self {
            Self::Sprite(d) => d.layer,
            Self::Rect(d) => d.layer,
            Self::Line(d) => d.layer,
            Self::Circle(d) => d.layer,
            Self::Text { layer, .. } => *layer,
        }
    }
}
