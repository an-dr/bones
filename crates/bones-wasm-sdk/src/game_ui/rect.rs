/// Axis-aligned logical rectangle used for layout and hit-testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width in logical pixels.
    pub width: u32,
    /// Height in logical pixels.
    pub height: u32,
}

impl Rect {
    /// Whether a logical point lies inside; the right and bottom edges are
    /// outside.
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x as f32
            && x < (self.x + self.width as i32) as f32
            && y >= self.y as f32
            && y < (self.y + self.height as i32) as f32
    }
}
