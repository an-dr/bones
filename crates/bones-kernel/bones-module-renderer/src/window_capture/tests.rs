use super::*;

#[test]
fn bgrx_pixels_become_opaque_rgba() {
    let mut pixels = vec![1, 2, 3, 0, 10, 20, 30, 7];
    bgrx_to_rgba(&mut pixels);
    assert_eq!(pixels, [3, 2, 1, 255, 30, 20, 10, 255]);
}
