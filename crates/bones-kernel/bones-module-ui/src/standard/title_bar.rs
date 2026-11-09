use bones_messages::ui::icons;
use bones_messages::ui::kinds::TitleBar;

use crate::chrome;
use crate::draw_context::DrawContext;
use crate::kind::Kind;

const HEIGHT: f32 = 36.0;
const BUTTON_WIDTH: f32 = 46.0;
const BUTTONS: f32 = 3.0;

pub(crate) struct TitleBarKind;

impl Kind for TitleBarKind {
    type Props = TitleBar;

    fn draw(&self, ui: &mut egui::Ui, _id: u32, props: TitleBar, cx: &mut DrawContext<'_>) {
        // Where the window keeps its system frame, a second bar would only
        // confuse; draw nothing.
        if !chrome::ensure_installed() {
            return;
        }
        let (bar, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), HEIGHT),
            egui::Sense::hover(),
        );
        let ppp = ui.ctx().pixels_per_point();
        chrome::publish_layout(bar.bottom() * ppp, BUTTON_WIDTH * BUTTONS * ppp);

        let mut text_left = bar.left() + 12.0;
        if let Some(texture) = cx.image(props.icon) {
            let rect = egui::Rect::from_center_size(
                bar.left_center() + egui::vec2(22.0, 0.0),
                egui::vec2(18.0, 18.0),
            );
            egui::Image::new(texture)
                .corner_radius(4)
                .paint_at(ui, rect);
            text_left = bar.left() + 40.0;
        }
        ui.painter().text(
            egui::pos2(text_left, bar.center().y),
            egui::Align2::LEFT_CENTER,
            props.title,
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().strong_text_color(),
        );

        let at = |index: f32| {
            egui::Rect::from_min_size(
                egui::pos2(bar.right() - BUTTON_WIDTH * (BUTTONS - index), bar.top()),
                egui::vec2(BUTTON_WIDTH, bar.height()),
            )
        };
        let maximize = if chrome::is_maximized() {
            icons::COPY
        } else {
            icons::SQUARE
        };
        if caption(ui, at(0.0), icons::MINUS, false) {
            chrome::minimize();
        }
        if caption(ui, at(1.0), maximize, false) {
            chrome::toggle_maximize();
        }
        if caption(ui, at(2.0), icons::X, true) {
            chrome::close();
        }
    }
}

/// One caption button; the close button turns red under the pointer, as
/// desktops draw it.
fn caption(ui: &mut egui::Ui, rect: egui::Rect, glyph: &str, danger: bool) -> bool {
    let response = ui.interact(rect, ui.id().with(("caption", glyph)), egui::Sense::click());
    let mut ink = ui.visuals().text_color();
    if response.hovered() {
        if danger {
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_rgb(196, 43, 28));
            ink = egui::Color32::WHITE;
        } else {
            let fill = ui.visuals().widgets.hovered.weak_bg_fill;
            ui.painter().rect_filled(rect, 0.0, fill);
        }
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(14.0),
        ink,
    );
    response.clicked()
}
