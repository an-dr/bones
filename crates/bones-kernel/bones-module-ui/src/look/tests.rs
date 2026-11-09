use egui::Color32;

use super::desktop::{from_xdg, Desktop};
use super::style::{luminance, metrics, on_colour, readable_on, visuals};
use super::system::{
    gnome_accent, gnome_font, kde_font, kde_section_value, parse_rgb, windows_accent,
};

const DESKTOPS: [Desktop; 4] = [
    Desktop::Windows,
    Desktop::Kde,
    Desktop::Gnome,
    Desktop::Cosmic,
];

#[test]
fn reads_the_desktop_from_xdg() {
    assert_eq!(from_xdg("ubuntu:GNOME"), Some(Desktop::Gnome));
    assert_eq!(from_xdg("KDE"), Some(Desktop::Kde));
    assert_eq!(from_xdg("COSMIC"), Some(Desktop::Cosmic));
    assert_eq!(from_xdg("XFCE"), None);
}

#[test]
fn parses_system_values() {
    assert_eq!(gnome_font("'Cantarell 11'").as_deref(), Some("Cantarell"));
    assert_eq!(
        gnome_font("'Adwaita Sans 10.5'").as_deref(),
        Some("Adwaita Sans")
    );
    assert_eq!(
        kde_font("Noto Sans,10,-1,5,50,0").as_deref(),
        Some("Noto Sans")
    );
    assert_eq!(
        gnome_accent("'teal'\n"),
        Some(Color32::from_rgb(33, 144, 164))
    );
    assert_eq!(gnome_accent("'mauve'"), None);
    assert_eq!(
        parse_rgb("61, 174,233"),
        Some(Color32::from_rgb(61, 174, 233))
    );
    assert_eq!(parse_rgb("61,174"), None);
    assert_eq!(
        windows_accent(0xFF_D4_78_00),
        Color32::from_rgb(0, 120, 212)
    );
}

#[test]
fn reads_only_the_named_kde_section() {
    let text = "[Colors:View]\nBackgroundNormal=1,2,3\n[Colors:Window]\nBackgroundNormal=4,5,6\n";
    assert_eq!(
        kde_section_value(text, "Colors:Window", "BackgroundNormal").as_deref(),
        Some("4,5,6")
    );
    assert_eq!(
        kde_section_value(text, "Colors:Nope", "BackgroundNormal"),
        None
    );
}

#[test]
fn text_contrasts_with_panels_on_every_desktop_and_mode() {
    for desktop in DESKTOPS {
        for dark in [false, true] {
            let look = metrics(desktop);
            let v = visuals(&look, dark, look.accent);
            let text = v.widgets.noninteractive.fg_stroke.color;
            assert!((luminance(text) - luminance(v.panel_fill)).abs() > 0.6);
            assert_eq!(v.selection.stroke.color, on_colour(look.accent));
            assert_eq!(v.dark_mode, dark);
            assert!((luminance(v.hyperlink_color) - luminance(v.panel_fill)).abs() >= 0.2);
        }
    }
}

#[test]
fn a_low_contrast_accent_is_adjusted_for_links() {
    let navy = Color32::from_rgb(10, 20, 60);
    assert_ne!(readable_on(navy, Color32::from_gray(30)), navy);
    assert_eq!(readable_on(navy, Color32::from_gray(240)), navy);
}
