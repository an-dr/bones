use super::*;

#[test]
fn default_is_rect() {
    assert_eq!(Shape::default(), Shape::Rect);
}
