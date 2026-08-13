/// A reusable screen-space facing value for 2D objects. Positive x points
/// right and positive y points down, matching bones world coordinates.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFacing {
    Up,
    UpRight,
    Right,
    DownRight,
    #[default]
    Down,
    DownLeft,
    Left,
    UpLeft,
}

impl ObjectFacing {
    /// Classifies a velocity into the four cardinal directions. The dominant
    /// axis wins and exact diagonal ties are horizontal.
    pub fn cardinal_from_velocity(vx: f32, vy: f32) -> Option<Self> {
        if vx == 0.0 && vy == 0.0 {
            return None;
        }
        if vx.abs() >= vy.abs() {
            Some(if vx < 0.0 { Self::Left } else { Self::Right })
        } else {
            Some(if vy < 0.0 { Self::Up } else { Self::Down })
        }
    }

    /// Classifies a velocity into eight 45-degree sectors. The diagonal
    /// sectors begin halfway between their neighboring cardinal axes.
    pub fn octagonal_from_velocity(vx: f32, vy: f32) -> Option<Self> {
        if vx == 0.0 && vy == 0.0 {
            return None;
        }

        const TAN_22_5_DEGREES: f32 = 0.414_213_57;
        const TAN_67_5_DEGREES: f32 = 2.414_213_7;

        let slope = if vx == 0.0 {
            f32::INFINITY
        } else {
            vy.abs() / vx.abs()
        };
        if slope <= TAN_22_5_DEGREES {
            return Some(if vx < 0.0 { Self::Left } else { Self::Right });
        }
        if slope >= TAN_67_5_DEGREES {
            return Some(if vy < 0.0 { Self::Up } else { Self::Down });
        }

        Some(match (vx < 0.0, vy < 0.0) {
            (false, true) => Self::UpRight,
            (false, false) => Self::DownRight,
            (true, false) => Self::DownLeft,
            (true, true) => Self::UpLeft,
        })
    }
}

#[cfg(test)]
mod tests;
