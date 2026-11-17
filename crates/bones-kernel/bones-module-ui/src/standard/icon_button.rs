use bones_messages::ui::icons;
use bones_messages::ui::kinds::IconButton;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct IconButtonKind;

impl Kind for IconButtonKind {
    type Props = IconButton;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: IconButton, cx: &mut DrawContext<'_>) {
        let icon_only = props.label.is_empty();
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
            button = egui::Button::new(text.clone()).fill(accent);
        }
        if icon_only {
            // Square, and in a layout of its own: a justified parent, such as a
            // side panel, would otherwise stretch it to the full width.
            let glyph = egui::WidgetText::from(text.clone())
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    egui::TextStyle::Button,
                )
                .size();
            let padding = ui.spacing().button_padding;
            let side = (glyph.x + 2.0 * padding.x)
                .max(glyph.y + 2.0 * padding.y)
                .max(ui.spacing().interact_size.y);
            button = button.min_size(egui::vec2(side, side));
        }
        let enabled = !props.disabled;
        let response = if icon_only {
            // One row: `with_layout` would take all the height left and
            // centre the button in it, pushing everything after it away.
            ui.horizontal(|ui| super::highlight::button(ui, |ui| ui.add_enabled(enabled, button)))
                .inner
        } else {
            super::highlight::button(ui, |ui| ui.add_enabled(enabled, button))
        };
        if response.clicked() {
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
