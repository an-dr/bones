use bones_messages::ui::icons;
use bones_messages::ui::kinds::Icon;
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct IconKind;

impl Kind for IconKind {
    type Props = Icon;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Icon, cx: &mut DrawContext<'_>) {
        let Some(glyph) = icons::by_name(&props.name) else {
            return;
        };
        let mut text = egui::RichText::new(glyph);
        if props.size > 0.0 {
            text = text.size(props.size);
        }
        let sense = match id {
            0 => egui::Sense::hover(),
            _ => egui::Sense::click(),
        };
        if ui.add(egui::Label::new(text).sense(sense)).clicked() {
            cx.emit(Event::Clicked { id });
        }
    }
}
