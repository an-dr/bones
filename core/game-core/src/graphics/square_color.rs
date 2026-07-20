/// Marks an entity as drawing a plain filled square (no sprite) at its
/// collider's extent — obstacles and walls that don't need art.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SquareColor(pub (u8, u8, u8, u8));

#[cfg(test)]
mod tests;
