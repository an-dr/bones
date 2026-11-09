use bones_messages::ui::kinds::Fit;

use super::placement;

fn rect(w: f32, h: f32) -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))
}

#[test]
fn cover_crops_the_long_side_and_fills_the_box() {
    let (painted, wide) = placement(Fit::Cover, rect(100.0, 100.0), egui::vec2(200.0, 100.0));
    assert_eq!(painted, rect(100.0, 100.0));
    assert_eq!(wide.size(), egui::vec2(0.5, 1.0));
    let (_, tall) = placement(Fit::Cover, rect(100.0, 100.0), egui::vec2(100.0, 200.0));
    assert_eq!(tall.size(), egui::vec2(1.0, 0.5));
}

#[test]
fn contain_fits_inside_and_centres() {
    let (painted, uv) = placement(Fit::Contain, rect(100.0, 100.0), egui::vec2(200.0, 100.0));
    assert_eq!(painted.size(), egui::vec2(100.0, 50.0));
    assert_eq!(painted.center(), egui::pos2(50.0, 50.0));
    assert_eq!(uv.size(), egui::vec2(1.0, 1.0));
}

#[test]
fn fill_stretches_to_the_box() {
    let (painted, uv) = placement(Fit::Fill, rect(40.0, 90.0), egui::vec2(200.0, 100.0));
    assert_eq!(painted, rect(40.0, 90.0));
    assert_eq!(uv.size(), egui::vec2(1.0, 1.0));
}
