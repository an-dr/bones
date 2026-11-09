use bones_messages::ui::kinds::Strong;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct StrongKind;

impl Kind for StrongKind {
    type Props = Strong;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: Strong, _cx: &mut DrawContext<'_>) {
        ui.strong(props.text);
    }
}
