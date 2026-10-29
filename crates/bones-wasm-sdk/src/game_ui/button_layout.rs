use crate::game_ui::Rect;

/// One button after a menu layout assigns its logical bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonLayout {
    /// The id of the [`Button`](crate::game_ui::Button) this was laid out from.
    pub id: u32,
    /// The text drawn on the button.
    pub label: String,
    /// Where the button sits, in logical canvas pixels.
    pub bounds: Rect,
}
