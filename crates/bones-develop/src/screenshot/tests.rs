use super::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| arg.to_string()).collect()
}

#[test]
fn no_options_take_one_frame_into_screenshot_png() {
    let options = parse(&[]).unwrap();
    assert_eq!(options.out, PathBuf::from("screenshot.png"));
    assert_eq!(options.frames, 1);
    assert!(options.presses.is_empty());
}

#[test]
fn presses_keep_their_order() {
    let options = parse(&args(&[
        "--press", "Space", "--frames", "30", "--press", "Left", "--out", "a.png",
    ]))
    .unwrap();
    assert_eq!(options.presses, ["Space", "Left"]);
    assert_eq!(options.frames, 30);
    assert_eq!(options.out, PathBuf::from("a.png"));
}

#[test]
fn window_is_a_flag_without_a_value() {
    let options = parse(&args(&["--window", "--frames", "2"])).unwrap();
    assert!(options.window);
    assert_eq!(options.frames, 2);
    assert!(!parse(&[]).unwrap().window);
}

#[test]
fn bad_options_say_what_is_wrong() {
    assert!(parse(&args(&["--frames", "0"]))
        .unwrap_err()
        .contains("--frames"));
    assert!(parse(&args(&["--frames"])).unwrap_err().contains("usage"));
    assert!(parse(&args(&["--zoom", "2"]))
        .unwrap_err()
        .contains("usage"));
}

#[test]
fn the_size_comes_from_the_png_header() {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend(800u32.to_be_bytes());
    png.extend(600u32.to_be_bytes());
    assert_eq!(png_size(&png), Some((800, 600)));
    assert_eq!(png_size(b"short"), None);
}
