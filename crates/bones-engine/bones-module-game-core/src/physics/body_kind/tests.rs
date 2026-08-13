use super::*;

#[test]
fn default_is_dynamic() {
    assert_eq!(BodyKind::default(), BodyKind::Dynamic);
}
