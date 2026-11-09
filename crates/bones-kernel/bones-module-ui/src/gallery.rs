//! Justified rows, from crossWall: items keep their shape, and each full
//! row fills the width.

const MIN_ASPECT: f32 = 0.5;
const MAX_ASPECT: f32 = 2.6;
/// For an item with no size.
pub(crate) const DEFAULT_ASPECT: f32 = 1.6;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Row {
    pub(crate) height: f32,
    /// Item index and width, left to right.
    pub(crate) items: Vec<(usize, f32)>,
}

pub(crate) fn aspect(width: f32, height: f32) -> f32 {
    if width <= 0.0 || height <= 0.0 {
        return DEFAULT_ASPECT;
    }
    (width / height).clamp(MIN_ASPECT, MAX_ASPECT)
}

/// Fills rows at `target` height, then scales each full row to fit
/// `available`. The last row stays at `target` height, so one item cannot
/// grow to fill it.
pub(crate) fn rows(aspects: &[f32], available: f32, target: f32, gap: f32) -> Vec<Row> {
    let mut result = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut sum = 0.0;
    for (index, aspect) in aspects.iter().enumerate() {
        current.push(index);
        sum += aspect;
        let gaps = gap * (current.len() - 1) as f32;
        if sum * target + gaps >= available {
            let height = (available - gaps) / sum;
            result.push(row(&current, aspects, height));
            current.clear();
            sum = 0.0;
        }
    }
    if !current.is_empty() {
        result.push(row(&current, aspects, target));
    }
    result
}

fn row(indices: &[usize], aspects: &[f32], height: f32) -> Row {
    Row {
        height,
        items: indices.iter().map(|&i| (i, aspects[i] * height)).collect(),
    }
}

#[cfg(test)]
mod tests;
