use super::*;

#[test]
fn equal_coordinates_are_equal() {
    assert_eq!(Transform { x: 1.0, y: 2.0 }, Transform { x: 1.0, y: 2.0 });
}
