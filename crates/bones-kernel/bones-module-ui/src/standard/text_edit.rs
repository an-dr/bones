use bones_messages::ui::kinds::TextEdit;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct TextEditKind;

impl Kind for TextEditKind {
    type Props = TextEdit;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: TextEdit, cx: &mut DrawContext<'_>) {
        let mut text = props.text;
        let response = ui.text_edit_singleline(&mut text);
        if response.changed() {
            cx.emit(Event::Changed { id, text });
        }
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            cx.emit(Event::Submitted { id });
        }
    }
}
