use bones_messages::ui::kinds::Button;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct ButtonKind;

impl Kind for ButtonKind {
    type Props = Button;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Button, cx: &mut DrawContext<'_>) {
        let button = egui::Button::new(props.label);
        if ui.add_enabled(!props.disabled, button).clicked() {
            cx.emit(Event::Clicked { id });
        }
    }
}
