//! Animated highlights, the default for every clickable standard widget:
//! hovering fades the highlight in and out, and a selection fades between
//! entries instead of jumping.

/// How long a highlight takes to fade in or out.
pub(crate) const HIGHLIGHT_SECONDS: f32 = 0.15;

/// How far the pointer and the selection have brought the widget drawn next
/// toward its highlight, each from 0 to 1. Keyed by the widget's position in
/// `ui`, with whether the pointer is over it read from the rect it took on
/// the last pass, since its colours are chosen before it is laid out.
struct Progress {
    key: egui::Id,
    hover: f32,
    selected: f32,
}

impl Progress {
    fn next(ui: &egui::Ui, selected: bool) -> Self {
        let key = ui.next_auto_id().with("highlight");
        let ctx = ui.ctx();
        let rect = ctx.data(|data| data.get_temp::<egui::Rect>(key));
        // Nothing lights up under a drag in progress.
        let over = rect.is_some_and(|rect| ui.rect_contains_pointer(rect))
            && ui.is_enabled()
            && ctx.dragged_id().is_none();
        Self {
            key,
            hover: ctx.animate_bool_with_time(key.with("hover"), over, HIGHLIGHT_SECONDS),
            selected: ctx.animate_bool_with_time(key.with("selected"), selected, HIGHLIGHT_SECONDS),
        }
    }

    fn remember(&self, ui: &egui::Ui, response: &egui::Response) {
        ui.ctx()
            .data_mut(|data| data.insert_temp(self.key, response.rect));
    }
}

/// Adds a button-like widget whose fill fades to its hover colour.
pub(crate) fn button(
    ui: &mut egui::Ui,
    add: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    let progress = Progress::next(ui, false);
    let saved = ui.visuals().clone();
    let widgets = &mut ui.visuals_mut().widgets;
    let fill = lerp(
        widgets.inactive.weak_bg_fill,
        widgets.hovered.weak_bg_fill,
        progress.hover,
    );
    // Both states take the blend, so egui's own switch at the hover edge
    // does not show.
    widgets.inactive.weak_bg_fill = fill;
    widgets.hovered.weak_bg_fill = fill;
    let response = add(ui);
    *ui.visuals_mut() = saved;
    progress.remember(ui, &response);
    response
}

/// A selectable label whose hover and selection highlights fade.
pub(crate) fn selectable(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let progress = Progress::next(ui, selected);
    let saved = ui.visuals().clone();
    let visuals = ui.visuals_mut();
    let hover_fill = visuals.widgets.hovered.weak_bg_fill;
    let accent = visuals.selection.bg_fill;
    let ink = lerp(
        visuals.text_color(),
        visuals.selection.stroke.color,
        progress.selected,
    );
    let fill = lerp(
        lerp(egui::Color32::TRANSPARENT, hover_fill, progress.hover),
        accent,
        progress.selected,
    );
    let radius = visuals.widgets.inactive.corner_radius;
    // egui draws its own background only when selected or hovered, all at
    // once; it is turned off here and the blend painted underneath instead.
    for state in [
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.inactive,
    ] {
        state.weak_bg_fill = egui::Color32::TRANSPARENT;
        state.bg_stroke = egui::Stroke::NONE;
        state.fg_stroke.color = ink;
    }
    visuals.override_text_color = Some(ink);
    let background = ui.painter().add(egui::Shape::Noop);
    let response = ui.selectable_label(false, text);
    *ui.visuals_mut() = saved;
    ui.painter().set(
        background,
        egui::Shape::rect_filled(response.rect, radius, fill),
    );
    progress.remember(ui, &response);
    response
}

fn lerp(from: egui::Color32, to: egui::Color32, t: f32) -> egui::Color32 {
    from.lerp_to_gamma(to, t.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests;
