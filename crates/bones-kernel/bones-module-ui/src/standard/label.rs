use bones_messages::ui::kinds::Label;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct LabelKind;

impl Kind for LabelKind {
    type Props = Label;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: Label, _cx: &mut DrawContext<'_>) {
        ui.label(props.text);
    }
}
