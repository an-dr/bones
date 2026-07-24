use super::*;

#[test]
fn equal_colors_are_equal() {
    assert_eq!(SquareColor((1, 2, 3, 4)), SquareColor((1, 2, 3, 4)));
}
