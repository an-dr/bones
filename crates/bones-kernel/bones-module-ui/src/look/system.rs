//! Reading the desktop's own settings: light or dark mode, the accent
//! colour, and the interface font. Each read can start a process, so the
//! ui module calls them off the frame thread.

use std::path::PathBuf;

use egui::Color32;

use super::desktop::Desktop;
use super::style::luminance;

/// The desktop's current mode and accent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct System {
    pub(crate) dark: bool,
    pub(crate) accent: Option<Color32>,
}

/// Reads the current mode and accent. Anything unavailable falls back: light
/// mode, and the desktop's standard accent.
pub(crate) fn read_system(desktop: Desktop) -> System {
    match desktop {
        Desktop::Windows => read_windows(),
        Desktop::Gnome => System {
            dark: command_output(
                "gsettings",
                &["get", "org.gnome.desktop.interface", "color-scheme"],
            )
            .is_some_and(|v| v.contains("prefer-dark")),
            accent: command_output(
                "gsettings",
                &["get", "org.gnome.desktop.interface", "accent-color"],
            )
            .and_then(|v| gnome_accent(&v)),
        },
        Desktop::Kde => System {
            dark: kde_value("Colors:Window", "BackgroundNormal")
                .and_then(|v| parse_rgb(&v))
                .is_some_and(|c| luminance(c) < 0.5),
            accent: kde_value("Colors:Selection", "BackgroundNormal").and_then(|v| parse_rgb(&v)),
        },
        Desktop::Cosmic => System {
            dark: dirs::config_dir()
                .and_then(|d| {
                    std::fs::read_to_string(
                        d.join("cosmic/com.system76.CosmicTheme.Mode/v1/is_dark"),
                    )
                    .ok()
                })
                .is_none_or(|v| v.trim() == "true"),
            accent: None,
        },
    }
}

/// The desktop's interface font file, if it can be found and read.
pub(crate) fn read_font(desktop: Desktop) -> Option<Vec<u8>> {
    font_path(desktop).and_then(|path| std::fs::read(path).ok())
}

fn font_path(desktop: Desktop) -> Option<PathBuf> {
    match desktop {
        Desktop::Windows => {
            let root = std::env::var_os("WINDIR")
                .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
            Some(root.join("Fonts").join("segoeui.ttf")).filter(|p| p.exists())
        }
        Desktop::Kde => fc_match(
            &kde_value("General", "font")
                .and_then(|f| kde_font(&f))
                .unwrap_or("Noto Sans".into()),
        ),
        Desktop::Gnome => fc_match(
            &command_output(
                "gsettings",
                &["get", "org.gnome.desktop.interface", "font-name"],
            )
            .and_then(|text| gnome_font(&text))
            .unwrap_or("Cantarell".into()),
        ),
        Desktop::Cosmic => fc_match("Open Sans"),
    }
}

fn fc_match(family: &str) -> Option<PathBuf> {
    command_output("fc-match", &["-f", "%{file}", family])
        .map(PathBuf::from)
        .filter(|p| p.exists())
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `'Cantarell 11'` gives `Cantarell`.
pub(crate) fn gnome_font(value: &str) -> Option<String> {
    let name = value.trim().trim_matches('\'');
    let (family, size) = name.rsplit_once(' ')?;
    size.parse::<f32>().ok().map(|_| family.to_string())
}

/// `Noto Sans,10,-1,5,50,...` gives `Noto Sans`.
pub(crate) fn kde_font(value: &str) -> Option<String> {
    value
        .split(',')
        .next()
        .filter(|f| !f.is_empty())
        .map(String::from)
}

pub(crate) fn kde_section_value(text: &str, section: &str, key: &str) -> Option<String> {
    let header = format!("[{section}]");
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == header;
        } else if inside {
            if let Some((name, value)) = line.split_once('=') {
                if name.trim() == key {
                    return Some(value.trim().to_string());
                }
            }
        }
    }
    None
}

fn kde_value(section: &str, key: &str) -> Option<String> {
    let path = dirs::config_dir()?.join("kdeglobals");
    kde_section_value(&std::fs::read_to_string(path).ok()?, section, key)
}

pub(crate) fn parse_rgb(value: &str) -> Option<Color32> {
    let mut parts = value.split(',').map(|p| p.trim().parse::<u8>());
    Some(Color32::from_rgb(
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ))
}

/// GNOME's named accents, as libadwaita defines them for its light style.
pub(crate) fn gnome_accent(value: &str) -> Option<Color32> {
    Some(match value.trim().trim_matches('\'') {
        "blue" => Color32::from_rgb(53, 132, 228),
        "teal" => Color32::from_rgb(33, 144, 164),
        "green" => Color32::from_rgb(58, 148, 74),
        "yellow" => Color32::from_rgb(200, 136, 0),
        "orange" => Color32::from_rgb(237, 91, 0),
        "red" => Color32::from_rgb(230, 45, 66),
        "pink" => Color32::from_rgb(213, 97, 153),
        "purple" => Color32::from_rgb(145, 65, 172),
        "slate" => Color32::from_rgb(111, 131, 150),
        _ => return None,
    })
}

/// Windows stores the accent as 0xAABBGGRR.
#[cfg(any(windows, test))]
pub(crate) fn windows_accent(value: u32) -> Color32 {
    Color32::from_rgb(value as u8, (value >> 8) as u8, (value >> 16) as u8)
}

#[cfg(windows)]
fn read_windows() -> System {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let user = RegKey::predef(HKEY_CURRENT_USER);
    let light: Option<u32> = user
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_value("AppsUseLightTheme"))
        .ok();
    let accent: Option<u32> = user
        .open_subkey(r"Software\Microsoft\Windows\DWM")
        .and_then(|key| key.get_value("AccentColor"))
        .ok();
    System {
        dark: light == Some(0),
        accent: accent.map(windows_accent),
    }
}

#[cfg(not(windows))]
fn read_windows() -> System {
    System {
        dark: false,
        accent: None,
    }
}
