use bones_messages::ui::kinds::Select;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct SelectKind;

impl Kind for SelectKind {
    type Props = Select;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Select, cx: &mut DrawContext<'_>) {
        let shown = props
            .selected
            .and_then(|index| props.options.get(index as usize))
            .cloned()
            .unwrap_or(props.placeholder);
        // egui's combo box does not wrap in a wrapping row by itself, so
        // start the next row when this one will not fit.
        if ui.layout().main_wrap {
            let spacing = ui.spacing();
            let text = egui::WidgetText::from(shown.as_str()).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            );
            let needed = (text.size().x
                + spacing.icon_width
                + spacing.icon_spacing
                + 2.0 * spacing.button_padding.x)
                .max(spacing.combo_width);
            if needed > ui.available_size_before_wrap().x {
                ui.end_row();
            }
        }
        egui::ComboBox::from_id_salt(("select", id))
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (index, option) in props.options.iter().enumerate() {
                    let index = index as u32;
                    let chosen = props.selected == Some(index);
                    if ui.selectable_label(chosen, option).clicked() && !chosen {
                        cx.emit(Event::Selected { id, index });
                    }
                }
            });
    }
}
