use super::*;

#[test]
fn rgba_rows_encode_to_a_png_that_decodes_back() {
    let rgba = [255, 0, 0, 255, 0, 255, 0, 255];
    let png = encode_png(2, 1, &rgba).unwrap();

    assert_eq!(&png[..4], b"\x89PNG");
    let decoder = png::Decoder::new(std::io::Cursor::new(png));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    assert_eq!(pixels, rgba);
}
