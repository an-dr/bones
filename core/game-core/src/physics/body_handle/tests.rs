use super::*;

#[test]
fn equal_inner_values_compare_equal() {
    assert_eq!(BodyHandle(7), BodyHandle(7));
    assert_ne!(BodyHandle(7), BodyHandle(8));
}
