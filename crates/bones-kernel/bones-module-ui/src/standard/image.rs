use bones_messages::ui::kinds::{Fit, Image};
use bones_messages::ui::Event;

use crate::draw_context::DrawContext;
use crate::kind::Kind;

pub(crate) struct ImageKind;

impl Kind for ImageKind {
    type Props = Image;

    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Image, cx: &mut DrawContext<'_>) {
        let texture = cx.image(props.handle).cloned();
        let natural = texture
            .as_ref()
            .map_or(egui::Vec2::ZERO, |texture| texture.size_vec2());
        let size = egui::vec2(
            if props.width > 0.0 {
                props.width
            } else {
                natural.x
            },
            if props.height > 0.0 {
                props.height
            } else {
                natural.y
            },
        );
        let sense = match id {
            0 => egui::Sense::hover(),
            _ => egui::Sense::click(),
        };
        let (rect, response) = ui.allocate_exact_size(size, sense);
        let corner = egui::CornerRadius::same(props.corner.round().clamp(0.0, 255.0) as u8);
        match texture {
            Some(texture) => {
                let (rect, uv) = placement(props.fit, rect, natural);
                egui::Image::new(&texture)
                    .uv(uv)
                    .corner_radius(corner)
                    .maintain_aspect_ratio(false)
                    .paint_at(ui, rect);
            }
            None => {
                let fill = ui.visuals().faint_bg_color;
                ui.painter().rect_filled(rect, corner, fill);
            }
        }
        if response.clicked() {
            cx.emit(Event::Clicked { id });
        }
    }
}

/// Where in `rect` the image is painted, and which part of it (in texture
/// coordinates), for an image of `natural` size.
fn placement(fit: Fit, rect: egui::Rect, natural: egui::Vec2) -> (egui::Rect, egui::Rect) {
    let whole = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
    if natural.x <= 0.0 || natural.y <= 0.0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return (rect, whole);
    }
    let image_ratio = natural.x / natural.y;
    let box_ratio = rect.width() / rect.height();
    match fit {
        Fit::Fill => (rect, whole),
        Fit::Cover => {
            let (w, h) = if image_ratio > box_ratio {
                (box_ratio / image_ratio, 1.0)
            } else {
                (1.0, image_ratio / box_ratio)
            };
            let uv = egui::Rect::from_center_size(egui::pos2(0.5, 0.5), egui::vec2(w, h));
            (rect, uv)
        }
        Fit::Contain => {
            let scale = (rect.width() / natural.x).min(rect.height() / natural.y);
            let painted = egui::Rect::from_center_size(rect.center(), natural * scale);
            (painted, whole)
        }
    }
}

#[cfg(test)]
mod tests;
