use bones_messages::ui::kinds::TextEdit;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct TextEditKind;

impl Kind for TextEditKind {
    type Props = TextEdit;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: TextEdit, cx: &mut DrawContext<'_>) {
        let mut text = props.text;
        if ui.text_edit_singleline(&mut text).changed() {
            cx.emit(Event::Changed { id, text });
        }
    }
}
