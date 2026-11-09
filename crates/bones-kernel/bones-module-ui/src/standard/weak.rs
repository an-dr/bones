use bones_messages::ui::kinds::Weak;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct WeakKind;

impl Kind for WeakKind {
    type Props = Weak;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: Weak, _cx: &mut DrawContext<'_>) {
        ui.weak(props.text);
    }
}
