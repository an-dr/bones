use crate::game_ui::{Canvas, MenuLayout};

/// Wrapped keyboard selection shared by game menu screens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Selection {
    index: usize,
}

impl Selection {
    /// The selected item's position in the menu.
    pub const fn index(self) -> usize {
        self.index
    }

    /// Selects the first item.
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// Moves the selection by `direction` items, wrapping at both ends of a
    /// menu with `item_count` items.
    pub fn move_by(&mut self, item_count: usize, direction: i32) {
        self.index = if item_count == 0 {
            0
        } else {
            (self.index as i32 + direction).rem_euclid(item_count as i32) as usize
        };
    }

    /// The selected button's id, or `None` when the layout has no button at
    /// the selected position.
    pub fn selected_id(self, layout: &MenuLayout) -> Option<u32> {
        layout.buttons.get(self.index).map(|button| button.id)
    }

    /// Selects the button under a pointer given in physical window pixels,
    /// and returns its id; the selection is unchanged when none is there.
    pub fn hover(
        &mut self,
        layout: &MenuLayout,
        canvas: Canvas,
        physical_x: f32,
        physical_y: f32,
        window_size: (u32, u32),
    ) -> Option<u32> {
        let (index, id) = layout.hit_test(canvas, physical_x, physical_y, window_size)?;
        self.index = index;
        Some(id)
    }
}
