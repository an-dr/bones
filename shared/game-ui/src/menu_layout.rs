use crate::{ButtonLayout, Canvas, Rect};

/// Fully positioned panel and buttons for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuLayout {
    pub panel: Rect,
    pub buttons: Vec<ButtonLayout>,
}

impl MenuLayout {
    pub fn hit_test(
        &self,
        canvas: Canvas,
        physical_x: f32,
        physical_y: f32,
        window_size: (u32, u32),
    ) -> Option<(usize, u32)> {
        let (x, y) = canvas.logical_point(physical_x, physical_y, window_size)?;
        self.buttons
            .iter()
            .enumerate()
            .find(|(_, button)| button.bounds.contains(x, y))
            .map(|(index, button)| (index, button.id))
    }
}
