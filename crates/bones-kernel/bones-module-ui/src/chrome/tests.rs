use super::{zone, Zone};

fn at(x: i32, y: i32, maximized: bool) -> Zone {
    zone(x, y, 1000, 800, 40, 150, maximized)
}

#[test]
fn the_title_drags_except_over_its_buttons() {
    assert_eq!(at(300, 20, false), Zone::Drag);
    assert_eq!(at(900, 20, false), Zone::Normal);
    assert_eq!(at(300, 60, false), Zone::Normal);
}

#[test]
fn edges_and_corners_resize() {
    assert_eq!(at(0, 400, false), Zone::Resize(-1, 0));
    assert_eq!(at(999, 799, false), Zone::Resize(1, 1));
    assert_eq!(at(500, 0, false), Zone::Resize(0, -1));
}

#[test]
fn a_maximized_window_has_no_resize_edges() {
    assert_eq!(at(0, 400, true), Zone::Normal);
    assert_eq!(at(500, 0, true), Zone::Drag);
}
