use super::*;

#[test]
fn equal_handles_are_equal() {
    let body = BodyHandle(1);
    let collider = ColliderHandle(1);

    let a = Collider {
        body,
        collider,
        half_w: 1.0,
        half_h: 1.0,
    };
    let b = Collider {
        body,
        collider,
        half_w: 1.0,
        half_h: 1.0,
    };
    assert_eq!(a, b);
}
