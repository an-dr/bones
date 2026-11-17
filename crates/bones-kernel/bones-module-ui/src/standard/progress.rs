use bones_messages::ui::kinds::Progress;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct ProgressKind;

impl Kind for ProgressKind {
    type Props = Progress;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: Progress, _cx: &mut DrawContext<'_>) {
        // Always the bar and one caption line, whatever is shown, so a bar
        // that starts, gains a caption, or goes idle never moves what is below.
        let bar_height = 4.0;
        let font = egui::TextStyle::Small.resolve(ui.style());
        let caption_height = ui.fonts(|fonts| fonts.row_height(&font));
        let gap = ui.spacing().item_spacing.y;
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), bar_height + gap + caption_height),
            egui::Sense::hover(),
        );
        if props.fraction.is_nan() {
            return;
        }
        let track = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), bar_height));
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
            .rect_filled(track, radius, visuals.extreme_bg_color);
        let bar = egui::Rect::from_x_y_ranges(
            track.left() + track.width() * from..=track.left() + track.width() * to,
            track.y_range(),
        );
        ui.painter()
            .rect_filled(bar, radius, visuals.selection.bg_fill);
        if !props.text.is_empty() {
            ui.painter().text(
                rect.right_bottom(),
                egui::Align2::RIGHT_BOTTOM,
                props.text,
                font,
                visuals.weak_text_color(),
            );
        }
    }
}
