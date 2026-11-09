use bones_messages::ui::kinds::Checkbox;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct CheckboxKind;

impl Kind for CheckboxKind {
    type Props = Checkbox;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Checkbox, cx: &mut DrawContext<'_>) {
        let mut checked = props.checked;
        if ui.checkbox(&mut checked, props.label).changed() {
            cx.emit(Event::Toggled { id, on: checked });
        }
    }
}
