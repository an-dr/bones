use bones_messages::ui::kinds::Button;
use bones_messages::ui::Event;

use crate::kind::Kind;

pub(crate) struct ButtonKind;

impl Kind for ButtonKind {
    type Props = Button;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Button, events: &mut Vec<Event>) {
        if ui.button(props.label).clicked() {
            events.push(Event::Clicked { id });
        }
    }
}
