use crate::{Button, ButtonLayout, Canvas, MenuLayout, Rect};

/// Configurable centered vertical menu geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalMenu {
    pub canvas: Canvas,
    pub panel_width: u32,
    pub header_height: u32,
    pub padding: u32,
    pub button_height: u32,
    pub gap: u32,
}

impl VerticalMenu {
    pub fn panel_height(self, button_count: usize) -> u32 {
        self.header_height
            + self.padding
            + button_count as u32 * self.button_height
            + button_count.saturating_sub(1) as u32 * self.gap
    }

    pub fn layout(self, buttons: impl IntoIterator<Item = Button>) -> MenuLayout {
        let buttons: Vec<_> = buttons.into_iter().collect();
        let panel_height = self.panel_height(buttons.len());
        let panel = Rect {
            x: (self.canvas.width as i32 - self.panel_width as i32) / 2,
            y: (self.canvas.height as i32 - panel_height as i32) / 2,
            width: self.panel_width,
            height: panel_height,
        };
        let button_width = self
            .panel_width
            .saturating_sub(self.padding.saturating_mul(2));
        let buttons = buttons
            .into_iter()
            .enumerate()
            .map(|(index, button)| ButtonLayout {
                id: button.id,
                label: button.label,
                bounds: Rect {
                    x: panel.x + self.padding as i32,
                    y: panel.y
                        + self.header_height as i32
                        + index as i32 * (self.button_height + self.gap) as i32,
                    width: button_width,
                    height: self.button_height,
                },
            })
            .collect();
        MenuLayout { panel, buttons }
    }
}
