use super::*;

/// A PNG of `width` × `height` pixels, each `rgba`.
fn png_of(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let pixels: Vec<u8> = (0..width * height).flat_map(|_| rgba).collect();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    png
}

#[test]
fn pixels_read_back_by_position() {
    let shot = Screenshot::from_png(3, 2, png_of(3, 2, [10, 20, 30, 255])).unwrap();
    assert_eq!(shot.pixel(0, 0), Some([10, 20, 30, 255]));
    assert_eq!(shot.pixel(2, 1), Some([10, 20, 30, 255]));
    assert_eq!(shot.pixel(3, 0), None);
    assert_eq!(shot.pixel(0, 2), None);
}

#[test]
fn a_size_other_than_stated_is_an_error() {
    let error = Screenshot::from_png(4, 4, png_of(3, 2, [0; 4])).unwrap_err();
    assert!(error.contains("expected 4x4"), "{error}");
}

#[test]
fn not_a_png_is_an_error() {
    assert!(Screenshot::from_png(1, 1, b"nope".to_vec()).is_err());
}
