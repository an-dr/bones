use bones_messages::ui::kinds::Hyperlink;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct HyperlinkKind;

impl Kind for HyperlinkKind {
    type Props = Hyperlink;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: Hyperlink, _cx: &mut DrawContext<'_>) {
        ui.hyperlink_to(props.label, props.url);
    }
}
