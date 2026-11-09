use bones_messages::ui::icons;
use bones_messages::ui::kinds::IconButton;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct IconButtonKind;

impl Kind for IconButtonKind {
    type Props = IconButton;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: IconButton, cx: &mut DrawContext<'_>) {
        let caption = match icons::by_name(&props.icon) {
            Some(glyph) if props.label.is_empty() => glyph.to_string(),
            Some(glyph) => format!("{glyph}  {}", props.label),
            None => props.label,
        };
        let mut text = egui::RichText::new(caption);
        let mut button = egui::Button::new(text.clone());
        if props.primary {
            let accent = ui.visuals().selection.bg_fill;
            text = text.color(ink_on(accent));
            button = egui::Button::new(text).fill(accent);
        }
        if ui.add_enabled(!props.disabled, button).clicked() {
            cx.emit(Event::Clicked { id });
        }
    }
}

/// Black or white, whichever reads better on `fill`.
fn ink_on(fill: egui::Color32) -> egui::Color32 {
    let [r, g, b, _] = fill.to_normalized_gamma_f32();
    let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if luminance > 0.55 {
        egui::Color32::BLACK
    } else {
        egui::Color32::WHITE
    }
}
