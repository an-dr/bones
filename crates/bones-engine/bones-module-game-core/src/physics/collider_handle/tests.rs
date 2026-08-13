use super::*;

#[test]
fn equal_inner_values_compare_equal() {
    assert_eq!(ColliderHandle(3), ColliderHandle(3));
    assert_ne!(ColliderHandle(3), ColliderHandle(4));
}
