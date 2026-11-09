use bones_messages::ui::kinds::Space;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct SpaceKind;

impl Kind for SpaceKind {
    type Props = Space;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: Space, _cx: &mut DrawContext<'_>) {
        ui.add_space(props.amount);
    }
}
