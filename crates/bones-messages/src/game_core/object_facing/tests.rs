use super::*;

#[test]
fn default_facing_is_down() {
    assert_eq!(ObjectFacing::default(), ObjectFacing::Down);
}

#[test]
fn stationary_velocity_has_no_facing() {
    assert_eq!(ObjectFacing::cardinal_from_velocity(0.0, 0.0), None);
    assert_eq!(ObjectFacing::octagonal_from_velocity(0.0, 0.0), None);
}

#[test]
fn cardinal_classification_uses_the_dominant_axis_and_horizontal_ties() {
    assert_eq!(
        ObjectFacing::cardinal_from_velocity(-2.0, 1.0),
        Some(ObjectFacing::Left)
    );
    assert_eq!(
        ObjectFacing::cardinal_from_velocity(2.0, -1.0),
        Some(ObjectFacing::Right)
    );
    assert_eq!(
        ObjectFacing::cardinal_from_velocity(1.0, -2.0),
        Some(ObjectFacing::Up)
    );
    assert_eq!(
        ObjectFacing::cardinal_from_velocity(-1.0, 2.0),
        Some(ObjectFacing::Down)
    );
    assert_eq!(
        ObjectFacing::cardinal_from_velocity(-1.0, -1.0),
        Some(ObjectFacing::Left)
    );
}

#[test]
fn octagonal_classification_covers_every_direction() {
    for (velocity, expected) in [
        ((0.0, -1.0), ObjectFacing::Up),
        ((1.0, -1.0), ObjectFacing::UpRight),
        ((1.0, 0.0), ObjectFacing::Right),
        ((1.0, 1.0), ObjectFacing::DownRight),
        ((0.0, 1.0), ObjectFacing::Down),
        ((-1.0, 1.0), ObjectFacing::DownLeft),
        ((-1.0, 0.0), ObjectFacing::Left),
        ((-1.0, -1.0), ObjectFacing::UpLeft),
    ] {
        assert_eq!(
            ObjectFacing::octagonal_from_velocity(velocity.0, velocity.1),
            Some(expected)
        );
    }
}

#[test]
fn octagonal_classification_keeps_near_axis_vectors_cardinal() {
    assert_eq!(
        ObjectFacing::octagonal_from_velocity(10.0, 1.0),
        Some(ObjectFacing::Right)
    );
    assert_eq!(
        ObjectFacing::octagonal_from_velocity(1.0, -10.0),
        Some(ObjectFacing::Up)
    );
}
