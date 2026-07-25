/// Axis-aligned logical rectangle used for layout and hit-testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x as f32
            && x < (self.x + self.width as i32) as f32
            && y >= self.y as f32
            && y < (self.y + self.height as i32) as f32
    }
}
