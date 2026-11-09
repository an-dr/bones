use super::{aspect, rows, DEFAULT_ASPECT, MAX_ASPECT, MIN_ASPECT};

#[test]
fn full_rows_fill_the_width_exactly() {
    let aspects = [1.5, 1.0, 2.0, 0.7, 1.6, 1.6, 1.6];
    let laid_out = rows(&aspects, 1000.0, 200.0, 10.0);
    for row in &laid_out[..laid_out.len() - 1] {
        let used: f32 =
            row.items.iter().map(|t| t.1).sum::<f32>() + 10.0 * (row.items.len() - 1) as f32;
        assert!((used - 1000.0).abs() < 0.01, "{used}");
    }
    let order: Vec<usize> = laid_out
        .iter()
        .flat_map(|r| r.items.iter().map(|t| t.0))
        .collect();
    assert_eq!(order, [0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn a_short_last_row_is_not_stretched() {
    let laid_out = rows(&[1.0], 1000.0, 200.0, 10.0);
    assert_eq!(laid_out[0].height, 200.0);
    assert_eq!(laid_out[0].items[0].1, 200.0);
}

#[test]
fn extreme_or_unknown_shapes_are_bounded() {
    assert_eq!(aspect(10000.0, 100.0), MAX_ASPECT);
    assert_eq!(aspect(100.0, 10000.0), MIN_ASPECT);
    assert_eq!(aspect(0.0, 0.0), DEFAULT_ASPECT);
}

#[test]
fn a_narrow_window_still_lays_out() {
    let laid_out = rows(&[1.6, 1.6], 100.0, 200.0, 10.0);
    assert_eq!(laid_out.len(), 2);
    assert!((laid_out[0].items[0].1 - 100.0).abs() < 0.01);
}
