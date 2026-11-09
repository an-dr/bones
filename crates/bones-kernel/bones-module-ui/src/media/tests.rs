use super::cover_uv;

#[test]
fn a_tile_crops_the_long_side_of_its_image() {
    let wide = cover_uv(egui::vec2(200.0, 100.0), egui::vec2(100.0, 100.0));
    assert_eq!(wide.size(), egui::vec2(0.5, 1.0));
    let tall = cover_uv(egui::vec2(100.0, 200.0), egui::vec2(100.0, 100.0));
    assert_eq!(tall.size(), egui::vec2(1.0, 0.5));
    assert_eq!(wide.center(), egui::pos2(0.5, 0.5));
}
