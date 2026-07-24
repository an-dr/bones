/// A looping frame-index-from-elapsed-time animation over a sprite's
/// source rectangle (ADR-019: built directly, no crate warranted).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteAnimation {
    pub sprite_id: u32,
    pub frame_w: u32,
    pub frame_h: u32,
    pub frame_count: u32,
    pub frame_duration: f32,
    elapsed: f32,
}

impl SpriteAnimation {
    pub fn new(
        sprite_id: u32,
        frame_w: u32,
        frame_h: u32,
        frame_count: u32,
        frame_duration: f32,
    ) -> Self {
        Self {
            sprite_id,
            frame_w,
            frame_h,
            frame_count,
            frame_duration,
            elapsed: 0.0,
        }
    }

    /// Advances the loop by `dt` seconds. A `frame_count` of `0` or `1`, or
    /// a non-positive `frame_duration`, never advances (a static sprite).
    pub fn advance(&mut self, dt: f32) {
        if self.frame_count <= 1 || self.frame_duration <= 0.0 {
            return;
        }
        let loop_duration = self.frame_duration * self.frame_count as f32;
        self.elapsed = (self.elapsed + dt) % loop_duration;
    }

    /// The frame currently showing, derived from elapsed time rather than
    /// stored directly — reconstructible at any point, never drifts.
    pub fn current_frame(&self) -> u32 {
        if self.frame_count <= 1 || self.frame_duration <= 0.0 {
            return 0;
        }
        (self.elapsed / self.frame_duration) as u32 % self.frame_count
    }

    /// The source-rectangle x offset for `current_frame`, in sprite-sheet
    /// pixels — frames are laid out left to right at `frame_w` spacing.
    pub fn current_src_x(&self) -> i32 {
        (self.current_frame() * self.frame_w) as i32
    }
}

#[cfg(test)]
mod tests;
