//! The native look (ADR-049): the ui module follows the desktop's font,
//! light or dark mode, accent colour, and spacing, and keeps following
//! them while it runs. It resembles the platform; it does not wrap the
//! platform's controls.

mod desktop;
mod style;
mod system;

use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use bones_kernel_infra::ui_port::Theme;

use crate::fonts;
use desktop::Desktop;
use system::System;

/// How often the desktop's mode and accent are read again.
const SYSTEM_POLL: Duration = Duration::from_secs(5);

/// How long a change between light and dark fades, in seconds.
const FADE_SECONDS: f64 = 0.18;

/// The look the ui module is following, and what it last applied.
pub(crate) struct NativeLook {
    desktop: Desktop,
    system: System,
    updates: Receiver<System>,
    applied: Option<(bool, Option<egui::Color32>)>,
    /// The panel colour before a light/dark change, and when it changed.
    fade: Option<(egui::Color32, f64)>,
}

impl NativeLook {
    /// Reads the desktop now, so the first frame is already right, then
    /// keeps reading it on threads of its own: the mode and accent every few
    /// seconds, the font once (both can start processes).
    pub(crate) fn start(ctx: &egui::Context) -> Self {
        let desktop = Desktop::detect();
        let system = system::read_system(desktop);
        let (sender, updates) = channel();
        std::thread::spawn(move || loop {
            std::thread::sleep(SYSTEM_POLL);
            if sender.send(system::read_system(desktop)).is_err() {
                return;
            }
        });
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if let Some(font) = system::read_font(desktop) {
                ctx.set_fonts(fonts::definitions(Some(font)));
            }
        });
        Self {
            desktop,
            system,
            updates,
            applied: None,
            fade: None,
        }
    }

    /// Applies the look when the system or the owner's `theme` changed since
    /// the last frame.
    pub(crate) fn follow(&mut self, ctx: &egui::Context, theme: Theme, time: f64) {
        while let Ok(system) = self.updates.try_recv() {
            self.system = system;
        }
        let dark = match theme {
            Theme::System => self.system.dark,
            Theme::Light => false,
            Theme::Dark => true,
        };
        let wanted = (dark, self.system.accent);
        if self.applied == Some(wanted) {
            return;
        }
        if self.applied.is_some_and(|(was_dark, _)| was_dark != dark) {
            self.fade = Some((ctx.style().visuals.panel_fill, time));
        }
        style::apply(ctx, self.desktop, dark, self.system.accent);
        self.applied = Some(wanted);
    }

    /// Covers the window in the old panel colour and lifts it, so a change
    /// between light and dark is not a flash. Drawn above everything.
    pub(crate) fn paint_fade(&mut self, ctx: &egui::Context, time: f64) {
        let Some((colour, started)) = self.fade else {
            return;
        };
        let linear = ((time - started) / FADE_SECONDS).clamp(0.0, 1.0) as f32;
        if linear >= 1.0 {
            self.fade = None;
            return;
        }
        let eased = 1.0 - (1.0 - linear).powi(3);
        let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("theme-fade"));
        ctx.layer_painter(layer).rect_filled(
            ctx.screen_rect(),
            0.0,
            colour.gamma_multiply(1.0 - eased),
        );
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests;
