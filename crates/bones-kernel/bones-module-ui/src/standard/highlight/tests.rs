use super::selectable;

/// The fill of the background `selectable` painted on one more pass.
fn fill_next(ctx: &egui::Context, selected: bool) -> egui::Color32 {
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            selectable(ui, selected, "Library");
        });
    });
    // The second rect: the panel paints its background first, and egui's own
    // label background, emptied, comes after.
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => Some(rect.fill),
            _ => None,
        })
        .nth(1)
        .expect("a background was painted")
}

#[test]
fn a_selection_fades_in_instead_of_jumping() {
    let ctx = egui::Context::default();
    assert_eq!(fill_next(&ctx, false), egui::Color32::TRANSPARENT);
    // egui moves animations a fixed step per pass, about a sixtieth of a second.
    let first = fill_next(&ctx, true);
    let middle = fill_next(&ctx, true);
    let mut end = middle;
    for _ in 0..30 {
        end = fill_next(&ctx, true);
    }
    assert_ne!(middle, first, "the fade did not move");
    assert_ne!(middle, end, "the selection jumped to its colour");
    assert_eq!(end, ctx.style().visuals.selection.bg_fill);
}
