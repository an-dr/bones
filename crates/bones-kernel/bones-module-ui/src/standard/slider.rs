use bones_messages::ui::kinds::Slider;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct SliderKind;

impl Kind for SliderKind {
    type Props = Slider;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Slider, cx: &mut DrawContext<'_>) {
        let mut value = props.value;
        if ui
            .add(egui::Slider::new(&mut value, props.min..=props.max))
            .changed()
        {
            cx.emit(Event::ValueChanged { id, value });
        }
    }
}
