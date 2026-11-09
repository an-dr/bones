//! Each desktop's spacing, corners, and colours, applied to egui.

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};

use super::desktop::Desktop;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Metrics {
    corner: u8,
    body: f32,
    button_padding: [f32; 2],
    item_spacing: [f32; 2],
    dark_panel: u8,
    light_panel: u8,
    pub(crate) accent: Color32,
}

pub(crate) fn metrics(desktop: Desktop) -> Metrics {
    match desktop {
        Desktop::Windows => Metrics {
            corner: 4,
            body: 14.0,
            button_padding: [11.0, 5.0],
            item_spacing: [8.0, 8.0],
            dark_panel: 32,
            light_panel: 243,
            accent: Color32::from_rgb(0, 120, 212),
        },
        Desktop::Kde => Metrics {
            corner: 3,
            body: 13.5,
            button_padding: [8.0, 5.0],
            item_spacing: [6.0, 6.0],
            dark_panel: 42,
            light_panel: 239,
            accent: Color32::from_rgb(61, 174, 233),
        },
        Desktop::Gnome => Metrics {
            corner: 9,
            body: 15.0,
            button_padding: [12.0, 7.0],
            item_spacing: [10.0, 10.0],
            dark_panel: 36,
            light_panel: 250,
            accent: Color32::from_rgb(53, 132, 228),
        },
        Desktop::Cosmic => Metrics {
            corner: 8,
            body: 14.0,
            button_padding: [12.0, 6.0],
            item_spacing: [8.0, 8.0],
            dark_panel: 27,
            light_panel: 242,
            accent: Color32::from_rgb(99, 208, 223),
        },
    }
}

/// Applies `desktop`'s colours and metrics in light or dark, with `accent`
/// or the desktop's standard one.
pub(crate) fn apply(ctx: &egui::Context, desktop: Desktop, dark: bool, accent: Option<Color32>) {
    let look = metrics(desktop);
    let accent = accent.unwrap_or(look.accent);
    ctx.set_visuals(visuals(&look, dark, accent));
    ctx.style_mut(|style| {
        let spacing = &mut style.spacing;
        spacing.item_spacing = look.item_spacing.into();
        spacing.button_padding = look.button_padding.into();
        spacing.interact_size.y = look.body + 2.0 * look.button_padding[1];
        spacing.window_margin = Margin::same(16);
        spacing.scroll = egui::style::ScrollStyle::floating();
        for (text, size, family) in [
            (TextStyle::Small, look.body * 0.85, FontFamily::Proportional),
            (TextStyle::Body, look.body, FontFamily::Proportional),
            (TextStyle::Button, look.body, FontFamily::Proportional),
            (
                TextStyle::Heading,
                look.body * 1.5,
                FontFamily::Proportional,
            ),
            (TextStyle::Monospace, look.body, FontFamily::Monospace),
        ] {
            style.text_styles.insert(text, FontId::new(size, family));
        }
    });
}

pub(crate) fn visuals(look: &Metrics, dark: bool, accent: Color32) -> egui::Visuals {
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let shade = |offset: i16| {
        let base = if dark {
            look.dark_panel
        } else {
            look.light_panel
        } as i16;
        let value = (base + if dark { offset } else { -offset }).clamp(0, 255) as u8;
        Color32::from_gray(value)
    };
    let text = if dark {
        Color32::from_gray(240)
    } else {
        Color32::from_gray(28)
    };
    let radius = CornerRadius::same(look.corner);
    v.panel_fill = shade(0);
    v.window_fill = shade(6);
    v.faint_bg_color = shade(6);
    v.extreme_bg_color = if dark { shade(-10) } else { Color32::WHITE };
    v.hyperlink_color = readable_on(accent, shade(0));
    v.selection.bg_fill = accent;
    v.selection.stroke = Stroke::new(1.0_f32, on_colour(accent));
    v.window_corner_radius = CornerRadius::same(look.corner.saturating_add(4));
    v.menu_corner_radius = radius;
    v.window_stroke = Stroke::new(1.0_f32, shade(22));
    for (state, fill) in [
        (&mut v.widgets.noninteractive, shade(0)),
        (&mut v.widgets.inactive, shade(16)),
        (&mut v.widgets.hovered, shade(26)),
        (&mut v.widgets.active, shade(34)),
        (&mut v.widgets.open, shade(26)),
    ] {
        state.corner_radius = radius;
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.expansion = 0.0;
        state.fg_stroke = Stroke::new(1.0_f32, text);
        state.bg_stroke = Stroke::NONE;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, shade(16));
    v
}

/// A dark accent is unreadable as link text on a dark panel, and the reverse.
pub(crate) fn readable_on(accent: Color32, panel: Color32) -> Color32 {
    if (luminance(accent) - luminance(panel)).abs() >= 0.3 {
        return accent;
    }
    let toward: f32 = if luminance(panel) < 0.5 { 255.0 } else { 0.0 };
    let mix = |c: u8| (c as f32 * 0.4 + toward * 0.6) as u8;
    Color32::from_rgb(mix(accent.r()), mix(accent.g()), mix(accent.b()))
}

/// Perceived brightness from 0 (black) to 1 (white).
pub(crate) fn luminance(c: Color32) -> f32 {
    (0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32) / 255.0
}

/// Black or white, whichever reads on `background`.
pub(crate) fn on_colour(background: Color32) -> Color32 {
    if luminance(background) > 0.55 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}
