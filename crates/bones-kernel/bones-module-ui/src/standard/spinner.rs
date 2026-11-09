use bones_messages::ui::kinds::Spinner;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct SpinnerKind;

impl Kind for SpinnerKind {
    type Props = Spinner;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, _props: Spinner, _cx: &mut DrawContext<'_>) {
        ui.spinner();
    }
}
