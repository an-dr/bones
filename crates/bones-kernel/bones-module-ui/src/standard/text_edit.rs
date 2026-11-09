use bones_messages::ui::kinds::TextEdit;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct TextEditKind;

impl Kind for TextEditKind {
    type Props = TextEdit;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: TextEdit, cx: &mut DrawContext<'_>) {
        let mut text = props.text;
        // egui's usual field width, or less where the row is narrower, so a
        // field in a wrapping row leaves its neighbours room to wrap.
        let width = ui.spacing().text_edit_width.min(ui.available_width());
        let field = egui::TextEdit::singleline(&mut text)
            .desired_width(width)
            .hint_text(props.hint)
            .margin(egui::Margin::symmetric(10, 5));
        let response = ui.add_enabled(!props.disabled, field);
        if props.focused && !response.has_focus() && !response.lost_focus() {
            response.request_focus();
        }
        if response.changed() {
            cx.emit(Event::Changed { id, text });
        }
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            cx.emit(Event::Submitted { id });
        }
    }
}
