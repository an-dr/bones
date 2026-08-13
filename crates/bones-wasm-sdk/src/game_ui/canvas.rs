/// Fixed logical coordinate space used by screen-space gfx commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
}

impl Canvas {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Converts physical window pixels into this logical canvas.
    pub fn logical_point(
        self,
        physical_x: f32,
        physical_y: f32,
        window_size: (u32, u32),
    ) -> Option<(f32, f32)> {
        let (window_width, window_height) = window_size;
        if window_width == 0 || window_height == 0 {
            return None;
        }
        Some((
            physical_x * self.width as f32 / window_width as f32,
            physical_y * self.height as f32 / window_height as f32,
        ))
    }
}
