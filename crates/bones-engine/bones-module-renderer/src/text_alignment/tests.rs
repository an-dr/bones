use super::*;

#[test]
fn alignment_offsets_from_the_requested_anchor() {
    assert_eq!(aligned_text_x(400, 120, TextAlign::Left), 400);
    assert_eq!(aligned_text_x(400, 120, TextAlign::Center), 340);
    assert_eq!(aligned_text_x(400, 120, TextAlign::Right), 280);
}

#[test]
fn alignment_saturates_instead_of_wrapping() {
    assert_eq!(
        aligned_text_x(i32::MIN, u32::MAX, TextAlign::Right),
        i32::MIN
    );
}
