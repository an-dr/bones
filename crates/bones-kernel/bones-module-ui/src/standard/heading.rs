use bones_messages::ui::kinds::Heading;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct HeadingKind;

impl Kind for HeadingKind {
    type Props = Heading;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: Heading, _cx: &mut DrawContext<'_>) {
        ui.heading(props.text);
    }
}
