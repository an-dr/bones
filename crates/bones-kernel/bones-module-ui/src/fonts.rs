/// The bundled Lucide icon font (ADR-049), ISC licence, beside this crate in
/// `assets/`. Brought in by `cargo xtask lucide`.
const LUCIDE: &[u8] = include_bytes!("../assets/lucide.ttf");

/// egui's fonts with `body` (the system font, when known) first and Lucide
/// as a fallback in every family, so `ui::icons` glyphs render inside any
/// text and the `icon` kind can draw by name.
pub(crate) fn definitions(body: Option<Vec<u8>>) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(body) = body {
        fonts
            .font_data
            .insert("system".to_owned(), egui::FontData::from_owned(body).into());
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.insert(0, "system".to_owned());
        }
    }
    fonts.font_data.insert(
        "lucide".to_owned(),
        egui::FontData::from_static(LUCIDE).into(),
    );
    for family in fonts.families.values_mut() {
        family.push("lucide".to_owned());
    }
    fonts
}
