use bones_messages::ui::kinds::Progress;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct ProgressKind;

impl Kind for ProgressKind {
    type Props = Progress;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: Progress, _cx: &mut DrawContext<'_>) {
        let height = 4.0;
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), height),
            egui::Sense::hover(),
        );
        let (from, to) = if props.fraction < 0.0 {
            // A short segment sweeping across, for work of unknown length.
            let time = ui.input(|i| i.time) as f32;
            ui.ctx().request_repaint();
            let start = (time * 0.7).fract() * 1.25 - 0.25;
            (start.max(0.0), (start + 0.25).min(1.0))
        } else {
            (0.0, props.fraction.min(1.0))
        };
        let visuals = ui.visuals();
        let radius = egui::CornerRadius::same(2);
        ui.painter()
            .rect_filled(rect, radius, visuals.extreme_bg_color);
        let bar = egui::Rect::from_x_y_ranges(
            rect.left() + rect.width() * from..=rect.left() + rect.width() * to,
            rect.y_range(),
        );
        ui.painter()
            .rect_filled(bar, radius, visuals.selection.bg_fill);
        if !props.text.is_empty() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.small(props.text);
            });
        }
    }
}
