use bones_messages::ui::icons;
use bones_messages::ui::kinds::Toast;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct ToastKind;

impl Kind for ToastKind {
    type Props = Toast;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Toast, cx: &mut DrawContext<'_>) {
        // Floats over the whole window wherever the tree places it.
        egui::Area::new(ui.id().with(("toast", id)))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(props.text);
                            let close = icons::X;
                            if ui.small_button(close).clicked() {
                                cx.emit(Event::Clicked { id });
                            }
                        });
                    });
            });
    }
}
