use bones_messages::ui::kinds::Selectable;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct SelectableKind;

impl Kind for SelectableKind {
    type Props = Selectable;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Selectable, cx: &mut DrawContext<'_>) {
        if ui.selectable_label(props.selected, props.label).clicked() {
            cx.emit(Event::Clicked { id });
        }
    }
}
