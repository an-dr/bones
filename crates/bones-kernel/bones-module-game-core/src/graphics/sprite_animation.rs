use bones_messages::game_core::SpritePresentation;

/// Frame-index-from-elapsed-time presentation over a sprite sheet
/// (ADR-019: built directly, no crate warranted).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteAnimation {
    pub sprite_id: u32,
    pub frame_w: u32,
    pub frame_h: u32,
    pub frame_count: u32,
    pub frame_duration: f32,
    pub frames_per_row: u32,
    pub draw_w: u32,
    pub draw_h: u32,
    pub looping: bool,
    pub advance_while_stopped: bool,
    pub flip_h: bool,
    pub flip_v: bool,
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
            frames_per_row: frame_count.max(1),
            draw_w: frame_w,
            draw_h: frame_h,
            looping: true,
            advance_while_stopped: false,
            flip_h: false,
            flip_v: false,
            elapsed: 0.0,
        }
    }

    pub fn from_presentation(presentation: SpritePresentation) -> Self {
        let sprite = presentation.sprite;
        Self {
            sprite_id: sprite.sprite_id,
            frame_w: sprite.frame_w,
            frame_h: sprite.frame_h,
            frame_count: sprite.frame_count,
            frame_duration: sprite.frame_duration,
            frames_per_row: if presentation.frames_per_row == 0 {
                sprite.frame_count.max(1)
            } else {
                presentation.frames_per_row
            },
            draw_w: presentation.draw_w,
            draw_h: presentation.draw_h,
            looping: presentation.looping,
            advance_while_stopped: presentation.advance_while_stopped,
            flip_h: presentation.flip_h,
            flip_v: presentation.flip_v,
            elapsed: 0.0,
        }
    }

    /// Advances playback by `dt` seconds. A `frame_count` of `0` or `1`, or
    /// a non-positive `frame_duration`, never advances (a static sprite).
    pub fn advance(&mut self, dt: f32) {
        if self.frame_count <= 1 || self.frame_duration <= 0.0 {
            return;
        }
        let loop_duration = self.frame_duration * self.frame_count as f32;
        self.elapsed = if self.looping {
            (self.elapsed + dt) % loop_duration
        } else {
            (self.elapsed + dt).min(loop_duration)
        };
    }

    /// The frame currently showing, derived from elapsed time rather than
    /// stored directly — reconstructible at any point, never drifts.
    pub fn current_frame(&self) -> u32 {
        if self.frame_count <= 1 || self.frame_duration <= 0.0 {
            return 0;
        }
        ((self.elapsed / self.frame_duration) as u32).min(self.frame_count - 1)
    }

    /// The source-rectangle x offset for `current_frame`, in sprite-sheet
    /// pixels.
    pub fn current_src_x(&self) -> i32 {
        (self.current_frame() % self.frames_per_row * self.frame_w) as i32
    }

    /// The source-rectangle y offset for the current grid frame.
    pub fn current_src_y(&self) -> i32 {
        (self.current_frame() / self.frames_per_row * self.frame_h) as i32
    }

    pub fn is_finished(&self) -> bool {
        !self.looping
            && self.frame_count > 1
            && self.frame_duration > 0.0
            && self.elapsed >= self.frame_duration * self.frame_count as f32
    }
}

#[cfg(test)]
mod tests;
