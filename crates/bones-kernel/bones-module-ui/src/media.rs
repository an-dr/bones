//! The media containers, generalised from crossWall: a `tile` showing an
//! image with actions on hover, and a full-window `stage` previewing one.

use bones_messages::ui::icons;

/// Seconds a newly arrived image or an opened stage takes to fade in.
const APPEAR_SECONDS: f32 = 0.25;

/// What a tile's drawing reported.
pub(crate) struct TileShown {
    /// Clicked outside its actions.
    pub(crate) clicked: bool,
    /// On screen for the first time.
    pub(crate) appeared: bool,
}

/// What the user asked of a stage.
pub(crate) enum StageAction {
    Close,
    /// 0 for previous, 1 for next.
    Step(u32),
}

/// Eased progress from 0 to 1 since `condition` became true under `id`.
fn appear(ui: &egui::Ui, id: egui::Id, condition: bool) -> f32 {
    let linear = ui
        .ctx()
        .animate_bool_with_time(id, condition, APPEAR_SECONDS);
    1.0 - (1.0 - linear).powi(3)
}

/// The part of an image of `natural` size that covers a box of `size`.
fn cover_uv(natural: egui::Vec2, size: egui::Vec2) -> egui::Rect {
    let image_ratio = natural.x / natural.y.max(1.0);
    let box_ratio = size.x / size.y.max(1.0);
    let (w, h) = if image_ratio > box_ratio {
        (box_ratio / image_ratio, 1.0)
    } else {
        (1.0, image_ratio / box_ratio)
    };
    egui::Rect::from_center_size(egui::pos2(0.5, 0.5), egui::vec2(w, h))
}

/// Draws a tile filling `size`.
pub(crate) fn tile(
    ui: &mut egui::Ui,
    id: egui::Id,
    size: egui::Vec2,
    texture: Option<egui::TextureHandle>,
    badge: u32,
    actions: impl FnOnce(&mut egui::Ui),
) -> TileShown {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let seen = id.with("seen");
    let appeared = ui.is_rect_visible(rect)
        && !ui
            .ctx()
            .data(|data| data.get_temp::<bool>(seen).unwrap_or(false));
    if appeared {
        ui.ctx().data_mut(|data| data.insert_temp(seen, true));
    }
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let radius = ui.visuals().widgets.inactive.corner_radius;
    let base = ui.visuals().faint_bg_color;
    match &texture {
        Some(texture) => {
            let shown = appear(ui, id.with("appear"), true);
            ui.painter().rect_filled(rect, radius, base);
            egui::Image::new(texture)
                .uv(cover_uv(texture.size_vec2(), size))
                .tint(egui::Color32::WHITE.gamma_multiply(shown))
                .corner_radius(radius)
                .maintain_aspect_ratio(false)
                .paint_at(ui, rect);
        }
        None => {
            // A soft pulse while the image loads, rather than a row of spinners.
            let time = ui.input(|i| i.time) as f32;
            ui.ctx().request_repaint();
            let pulse = 0.65 + 0.35 * (time * 2.5).sin().abs();
            ui.painter()
                .rect_filled(rect, radius, base.gamma_multiply(pulse));
        }
    }
    let hovered = ui.rect_contains_pointer(rect);
    let shown = appear(ui, id.with("hover"), hovered);
    if let Some(glyph) = char::from_u32(badge).filter(|_| badge != 0 && shown < 0.5) {
        let at = rect.right_top() + egui::vec2(-16.0, 16.0);
        ui.painter()
            .circle_filled(at, 11.0, egui::Color32::from_black_alpha(140));
        ui.painter().text(
            at,
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );
    }
    if shown > 0.0 {
        let bar = egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - 40.0),
            rect.right_bottom(),
        );
        let bar_radius = egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: radius.sw,
            se: radius.se,
        };
        ui.painter().rect_filled(
            bar,
            bar_radius,
            egui::Color32::from_black_alpha((170.0 * shown) as u8),
        );
    }
    if shown > 0.5 {
        let bar = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 8.0, rect.bottom() - 40.0),
            rect.right_bottom() - egui::vec2(8.0, 0.0),
        );
        let layout = egui::Layout::right_to_left(egui::Align::Center);
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt(id.with("actions"))
                .max_rect(bar)
                .layout(layout),
            |ui| {
                on_dark(ui);
                actions(ui);
            },
        );
    }
    TileShown {
        clicked: response.clicked(),
        appeared,
    }
}

/// White text and translucent buttons, for controls over a photo.
fn on_dark(ui: &mut egui::Ui) {
    let visuals = ui.visuals_mut();
    visuals.override_text_color = Some(egui::Color32::WHITE);
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_white_alpha(40);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_white_alpha(70);
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_white_alpha(90);
}

/// Draws a stage over the whole window.
pub(crate) fn stage(
    ui: &mut egui::Ui,
    id: egui::Id,
    texture: Option<egui::TextureHandle>,
    previous: bool,
    next: bool,
    bar: impl FnOnce(&mut egui::Ui),
) -> Option<StageAction> {
    let ctx = ui.ctx().clone();
    let screen = ctx.screen_rect();
    let opened = appear(ui, id.with("open"), true);
    let mut action = None;
    egui::Area::new(id.with("stage"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(&ctx, |ui| {
            // Takes the pointer, so nothing behind the stage reacts.
            ui.allocate_rect(screen, egui::Sense::click_and_drag());
            ui.painter().rect_filled(
                screen,
                0.0,
                egui::Color32::from_black_alpha((242.0 * opened) as u8),
            );
            let area = egui::Rect::from_min_max(
                screen.min + egui::vec2(64.0, 20.0),
                screen.max - egui::vec2(64.0, 76.0),
            );
            match &texture {
                Some(texture) => {
                    let natural = texture.size_vec2();
                    let fit = (area.width() / natural.x).min(area.height() / natural.y);
                    let grow = 0.97 + 0.03 * opened;
                    let rect = egui::Rect::from_center_size(area.center(), natural * fit * grow);
                    let radius = ui.visuals().widgets.inactive.corner_radius;
                    egui::Image::new(texture)
                        .tint(egui::Color32::WHITE.gamma_multiply(opened))
                        .corner_radius(radius)
                        .maintain_aspect_ratio(false)
                        .paint_at(ui, rect);
                }
                None => {
                    ui.put(
                        egui::Rect::from_center_size(area.center(), egui::vec2(32.0, 32.0)),
                        egui::Spinner::new().size(32.0),
                    );
                }
            }
            on_dark(ui);
            let round = |ui: &mut egui::Ui, centre: egui::Pos2, glyph: &str| {
                let rect = egui::Rect::from_center_size(centre, egui::vec2(38.0, 38.0));
                let text = egui::RichText::new(glyph).size(18.0);
                ui.put(rect, egui::Button::new(text).corner_radius(19))
                    .clicked()
            };
            if round(ui, screen.right_top() + egui::vec2(-32.0, 32.0), icons::X) {
                action = Some(StageAction::Close);
            }
            let middle = area.center().y;
            if previous
                && round(
                    ui,
                    egui::pos2(screen.left() + 32.0, middle),
                    icons::CHEVRON_LEFT,
                )
            {
                action = Some(StageAction::Step(0));
            }
            if next
                && round(
                    ui,
                    egui::pos2(screen.right() - 32.0, middle),
                    icons::CHEVRON_RIGHT,
                )
            {
                action = Some(StageAction::Step(1));
            }
            let strip = egui::Rect::from_min_max(
                egui::pos2(screen.left() + 24.0, screen.bottom() - 60.0),
                screen.right_bottom() - egui::vec2(24.0, 12.0),
            );
            let layout = egui::Layout::left_to_right(egui::Align::Center);
            ui.scope_builder(egui::UiBuilder::new().max_rect(strip).layout(layout), bar);
        });
    action
}

#[cfg(test)]
mod tests;
