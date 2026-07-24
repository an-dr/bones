//! Graphics: the drawable/visual-state side of an entity — world-space
//! transform, sprite-animation timing, and plain-square color.

mod sprite_animation;
mod square_color;
mod transform;

pub use sprite_animation::SpriteAnimation;
pub use square_color::SquareColor;
pub use transform::Transform;
