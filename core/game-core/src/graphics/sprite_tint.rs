/// RGBA color modulation applied to a sprite entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteTint(pub (u8, u8, u8, u8));

impl Default for SpriteTint {
    fn default() -> Self {
        Self((255, 255, 255, 255))
    }
}
