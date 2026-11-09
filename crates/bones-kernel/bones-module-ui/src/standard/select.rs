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
