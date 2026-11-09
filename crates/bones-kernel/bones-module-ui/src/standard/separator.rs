use bones_messages::ui::kinds::Separator;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct SeparatorKind;

impl Kind for SeparatorKind {
    type Props = Separator;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, _props: Separator, _cx: &mut DrawContext<'_>) {
        ui.separator();
    }
}
