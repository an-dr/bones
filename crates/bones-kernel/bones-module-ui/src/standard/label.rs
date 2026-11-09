use bones_messages::ui::kinds::Label;
use bones_messages::ui::Event;

use crate::kind::Kind;

pub(crate) struct LabelKind;

impl Kind for LabelKind {
    type Props = Label;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: Label, _events: &mut Vec<Event>) {
        ui.label(props.text);
    }
}
