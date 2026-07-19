use sdl3::render::FPoint;

// `radius` comes from an untrusted extension's `DrawCircle` payload. Both
// circle helpers below loop and do `2 * radius` arithmetic proportional to
// it; without a cap, a hostile or buggy extension could overflow that
// arithmetic or hang the render phase, which — unlike an extension — has no
// time budget (design/modules.md's trust model) and would stall the engine.
// Far larger than any real screen, so this never clips an intentional draw.
const MAX_CIRCLE_RADIUS: i32 = 10_000;

/// Points approximating a circle outline, centered on `(cx, cy)` with the
/// given `radius`, via the midpoint circle algorithm's 8-way symmetry.
pub(crate) fn circle_outline_points(cx: i32, cy: i32, radius: i32) -> Vec<FPoint> {
    let radius = radius.clamp(0, MAX_CIRCLE_RADIUS);
    let mut points = Vec::new();
    let mut x = radius;
    let mut y = 0;
    let mut err = 0;
    while x >= y {
        for (dx, dy) in [
            (x, y),
            (y, x),
            (-y, x),
            (-x, y),
            (-x, -y),
            (-y, -x),
            (y, -x),
            (x, -y),
        ] {
            points.push(FPoint::new((cx + dx) as f32, (cy + dy) as f32));
        }
        y += 1;
        err += 1 + 2 * y;
        if 2 * (err - x) + 1 > 0 {
            x -= 1;
            err += 1 - 2 * x;
        }
    }
    points
}

/// Horizontal scanlines (as endpoint pairs) filling a circle centered on
/// `(cx, cy)` with the given `radius`, one line per row.
pub(crate) fn circle_fill_lines(cx: i32, cy: i32, radius: i32) -> Vec<(FPoint, FPoint)> {
    let radius = radius.clamp(0, MAX_CIRCLE_RADIUS);
    let mut lines = Vec::with_capacity((2 * radius + 1) as usize);
    for dy in -radius..=radius {
        let half_width = ((radius * radius - dy * dy).max(0) as f32).sqrt().round() as i32;
        lines.push((
            FPoint::new((cx - half_width) as f32, (cy + dy) as f32),
            FPoint::new((cx + half_width) as f32, (cy + dy) as f32),
        ));
    }
    lines
}
