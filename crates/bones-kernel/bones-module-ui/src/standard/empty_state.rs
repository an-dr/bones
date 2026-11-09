use bones_messages::ui::icons;
use bones_messages::ui::kinds::EmptyState;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct EmptyStateKind;

impl Kind for EmptyStateKind {
    type Props = EmptyState;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: EmptyState, _cx: &mut DrawContext<'_>) {
        ui.add_space((ui.available_height() * 0.28).max(16.0));
        ui.vertical_centered(|ui| {
            if let Some(glyph) = icons::by_name(&props.icon) {
                let ink = ui.visuals().weak_text_color();
                ui.label(egui::RichText::new(glyph).size(52.0).color(ink));
                ui.add_space(6.0);
            }
            ui.heading(props.title);
            ui.weak(props.detail);
        });
    }
}
