/// World-space position of an entity — the `hecs` component every spawned
/// entity carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
mod tests;
