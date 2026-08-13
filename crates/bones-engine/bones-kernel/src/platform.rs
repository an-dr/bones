//! Platform layer (design/platform.md): the only component touching the OS.
//! Opens one SDL window, publishes keyboard, mouse, and gamepad events onto
//! `input/*`. TODO: tray and timing sources aren't published onto `input/*`
//! yet — text-input events are captured (`poll_events_with`) but only ever
//! reach the ui module's consumption hook, never the bus.

use std::collections::HashMap;

use crate::bus::{Bus, Envelope};
use bones_messages::input::{
    GamepadAxis, GamepadButtonDown, GamepadButtonUp, GamepadConnected, GamepadDisconnected,
    KeyDown, KeyUp, MouseDown, MouseMove, MouseUp, MouseWheel,
};
use bones_messages::{EncodeMessage, Message};

/// The value registered under the `window-surface` service: the OS window
/// itself, handed to whichever module claims it (design/modules.md).
///
/// An alias rather than a wrapper, for the reason
/// [`PlatformEvent`](crate::bus::PlatformEvent) is one: a module consuming
/// this service must name the exact type the provider registered, and it
/// should be able to do that through the engine's public surface instead of
/// depending on `sdl3` at a matching version.
pub type WindowSurface = sdl3::video::Window;

pub struct Platform {
    sdl: sdl3::Sdl,
    // `Option` so the window can be handed to a renderer later (`take_window`)
    // — event polling only needs the `Sdl` context, not this value itself.
    window: Option<sdl3::video::Window>,
    events: sdl3::EventPump,
    quit_requested: bool,
    gamepad_subsystem: sdl3::GamepadSubsystem,
    // A gamepad only generates axis/button events while its `Gamepad`
    // handle stays open (`SDL_OpenGamepad`) — closed automatically (`Drop`)
    // when removed here on `ControllerDeviceRemoved`.
    gamepads: HashMap<u32, sdl3::gamepad::Gamepad>,
    // Queried once here (the window's own display, before any hand-off) —
    // deduped/sorted unique (width, height) pairs from every fullscreen-
    // capable mode SDL reports (`SDL_GetFullscreenDisplayModes`, genuinely
    // cross-platform), collapsing the refresh-rate/pixel-density variants
    // a single resolution usually has several of. Empty if the query
    // itself failed (e.g. no display attached), not an error - a caller
    // building a resolution picker can always fall back to its own known
    // default instead.
    display_modes: Vec<(u32, u32)>,
    // The desktop's own current resolution (`SDL_GetDesktopDisplayMode`),
    // same query-time and failure-is-`None` caveats as `display_modes`.
    native_display_mode: Option<(u32, u32)>,
}

impl Platform {
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, String> {
        let sdl = sdl3::init().map_err(|e| e.to_string())?;
        let video = sdl.video().map_err(|e| e.to_string())?;
        let window = video
            .window(title, width, height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;
        // Started unconditionally, before the window can be handed away via
        // `take_window` — text input has no per-window "focused a text
        // field" signal yet, so this just keeps `Event::TextInput` flowing
        // for whichever layer (ui) wants to consume it. TODO: gate on focus
        // once one exists, for IME/mobile-keyboard hygiene.
        video.text_input().start(&window);
        let events = sdl.event_pump().map_err(|e| e.to_string())?;
        let gamepad_subsystem = sdl.gamepad().map_err(|e| e.to_string())?;

        let display = window.get_display().ok();
        let mut display_modes: Vec<(u32, u32)> = display
            .as_ref()
            .and_then(|display| display.get_fullscreen_modes().ok())
            .map(|modes| {
                modes
                    .iter()
                    .map(|mode| (mode.w as u32, mode.h as u32))
                    .collect()
            })
            .unwrap_or_default();
        display_modes.sort_unstable();
        display_modes.dedup();
        let native_display_mode = display
            .as_ref()
            .and_then(|display| display.get_mode().ok())
            .map(|mode| (mode.w as u32, mode.h as u32));

        Ok(Self {
            sdl,
            window: Some(window),
            events,
            quit_requested: false,
            gamepad_subsystem,
            gamepads: HashMap::new(),
            display_modes,
            native_display_mode,
        })
    }

    /// Floors how small the window can be resized, in addition to whatever
    /// size it opened at (`new`'s `width`/`height`). No-op if the window was
    /// already handed away via `take_window`.
    pub fn set_min_size(&mut self, width: u32, height: u32) -> Result<(), String> {
        match &mut self.window {
            Some(window) => window
                .set_minimum_size(width, height)
                .map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    /// Every fullscreen-capable resolution the window's own display
    /// reports, deduped and ascending — see this struct's own field doc
    /// comment. Empty, not an error, if the query failed.
    pub fn display_modes(&self) -> &[(u32, u32)] {
        &self.display_modes
    }

    /// The desktop's own current resolution, `None` if the query failed.
    pub fn native_display_mode(&self) -> Option<(u32, u32)> {
        self.native_display_mode
    }

    /// Hands the window over (e.g. to a renderer needing it for a `Canvas`).
    /// `None` if already taken. Keyboard input keeps working either way —
    /// `poll_events` only needs the `Sdl` context, never this value.
    pub fn take_window(&mut self) -> Option<sdl3::video::Window> {
        self.window.take()
    }

    /// Provides the window as the `window-surface` service (design/
    /// modules.md, ADR-017), for whichever module's `init` ends up
    /// consuming it — a no-op if already taken. The only place that ever
    /// provides a `Window`, so a duplicate-provide error here would mean a
    /// logic bug in `Engine::build`, not a normal runtime condition.
    pub fn provide_window(&mut self, services: &mut crate::bus::ServiceRegistry) {
        if let Some(window) = self.take_window() {
            services
                .provide(window)
                .expect("window-surface provided twice");
        }
    }

    /// Takes back an unclaimed `window-surface` service (nothing consumed
    /// it — e.g. `.window(...)` with no `.renderer()` and no custom module
    /// wanting it) so it stays open for the rest of the run, instead of
    /// being dropped — and closed — with the registry it briefly lived in.
    pub fn reclaim_window(&mut self, services: &mut crate::bus::ServiceRegistry) {
        if let Some(window) = services.consume::<sdl3::video::Window>() {
            self.window = Some(window);
        }
    }

    /// Publishes an `input/*` envelope for every pending keyboard event, and
    /// records a window-close request for `quit_requested` to report.
    /// Only enqueues (`Bus::publish`) — the caller decides when to dispatch.
    pub fn poll_events(&mut self, bus: &Bus, sender: &str) {
        self.poll_events_with(bus, sender, |_| false);
    }

    /// Same as `poll_events`, but offers every non-`Quit` raw event to
    /// `consumed` first (ADR-008: top layer consumes) — a higher layer
    /// (e.g. the ui module's egui context) that claims an event by
    /// returning `true` stops it from ever reaching `input/*`. `Quit` is a
    /// session-lifecycle signal, not layered input, so it always bypasses
    /// `consumed`.
    pub fn poll_events_with(
        &mut self,
        bus: &Bus,
        sender: &str,
        mut consumed: impl FnMut(&sdl3::event::Event) -> bool,
    ) {
        for event in self.events.poll_iter() {
            if matches!(event, sdl3::event::Event::Quit { .. }) {
                self.quit_requested = true;
                continue;
            }
            // Connection lifecycle, not layered input (ADR-008) — a UI
            // layer consuming a keypress shouldn't be able to hide a
            // gamepad connecting/disconnecting from a game, so this
            // bypasses `consumed` the same way `Quit` does. Also owns
            // opening/closing the `Gamepad` handle that axis/button events
            // depend on, so it can't be handled by the stateless
            // `translate_event` below.
            match &event {
                sdl3::event::Event::ControllerDeviceAdded { which, .. } => {
                    // The public `sdl3::joystick::JoystickId` is a type
                    // alias, not a constructible newtype, and the crate
                    // exposes no `From<u32>` for it — this reaches through
                    // the crate's own `pub extern crate ... as sys`
                    // re-export to build one, rather than patching sdl3.
                    // Silently drops a device this fails to open (no
                    // logger available in Platform to report it to).
                    if let Ok(gamepad) = self
                        .gamepad_subsystem
                        .open(sdl3::sys::joystick::SDL_JoystickID(*which))
                    {
                        self.gamepads.insert(*which, gamepad);
                        bus.publish(Envelope {
                            topic: GamepadConnected::TOPIC.to_string(),
                            sender: sender.to_string(),
                            correlation: None,
                            payload: GamepadConnected { id: *which }.encode(),
                        });
                    }
                    continue;
                }
                sdl3::event::Event::ControllerDeviceRemoved { which, .. } => {
                    self.gamepads.remove(which);
                    bus.publish(Envelope {
                        topic: GamepadDisconnected::TOPIC.to_string(),
                        sender: sender.to_string(),
                        correlation: None,
                        payload: GamepadDisconnected { id: *which }.encode(),
                    });
                    continue;
                }
                _ => {}
            }
            if consumed(&event) {
                continue;
            }
            if let Some(envelope) = translate_event(&event, sender) {
                bus.publish(envelope);
            }
        }
    }

    /// Whether the OS asked to close the window (e.g. the close button)
    /// since this `Platform` was created. Sticky — once true, stays true;
    /// the runner begins the orderly shutdown sequence instead of polling
    /// another frame.
    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// Injects a synthetic event into SDL's own queue, as if the OS had
    /// delivered it — for tests/tooling that simulate input without a real
    /// keypress. The next `poll_events` picks it up like any other event.
    /// For `KeyDown`/`KeyUp`, `scancode` must be `Some(..)` — SDL's own
    /// conversion needs it even though `translate_event` never reads it.
    pub fn inject_event(&self, event: sdl3::event::Event) -> Result<(), String> {
        self.sdl
            .event()
            .map_err(|e| e.to_string())?
            .push_event(event)
            .map_err(|e| e.to_string())
    }
}

fn translate_event(event: &sdl3::event::Event, sender: &str) -> Option<Envelope> {
    let (topic, payload) = match event {
        sdl3::event::Event::KeyDown {
            keycode: Some(key), ..
        } => {
            let key = key.to_string();
            (KeyDown::TOPIC, KeyDown { key: &key }.encode())
        }
        sdl3::event::Event::KeyUp {
            keycode: Some(key), ..
        } => {
            let key = key.to_string();
            (KeyUp::TOPIC, KeyUp { key: &key }.encode())
        }
        sdl3::event::Event::MouseMotion {
            x, y, xrel, yrel, ..
        } => (
            MouseMove::TOPIC,
            MouseMove {
                x: *x,
                y: *y,
                dx: *xrel,
                dy: *yrel,
            }
            .encode(),
        ),
        sdl3::event::Event::MouseButtonDown {
            mouse_btn, x, y, ..
        } => (
            MouseDown::TOPIC,
            MouseDown {
                button: *mouse_btn as u8,
                x: *x,
                y: *y,
            }
            .encode(),
        ),
        sdl3::event::Event::MouseButtonUp {
            mouse_btn, x, y, ..
        } => (
            MouseUp::TOPIC,
            MouseUp {
                button: *mouse_btn as u8,
                x: *x,
                y: *y,
            }
            .encode(),
        ),
        sdl3::event::Event::MouseWheel {
            x, y, direction, ..
        } => {
            // Normalize "flipped" (natural scrolling) so `input/mouse-wheel`
            // always has the same sign convention regardless of the OS
            // setting that produced it.
            let flip = matches!(direction, sdl3::mouse::MouseWheelDirection::Flipped);
            let (x, y) = if flip { (-x, -y) } else { (*x, *y) };
            (MouseWheel::TOPIC, MouseWheel { x, y }.encode())
        }
        sdl3::event::Event::ControllerAxisMotion {
            which, axis, value, ..
        } => {
            let axis = format!("{axis:?}");
            // SDL's raw range is roughly i16::MIN..=i16::MAX (sticks) or
            // 0..=i16::MAX (triggers); normalize against MAX and clamp so
            // the negative extreme (MIN < -MAX) never exceeds -1.0.
            let value = (*value as f32 / i16::MAX as f32).clamp(-1.0, 1.0);
            (
                GamepadAxis::TOPIC,
                GamepadAxis {
                    id: *which,
                    axis: &axis,
                    value,
                }
                .encode(),
            )
        }
        sdl3::event::Event::ControllerButtonDown { which, button, .. } => {
            let button = format!("{button:?}");
            (
                GamepadButtonDown::TOPIC,
                GamepadButtonDown {
                    id: *which,
                    button: &button,
                }
                .encode(),
            )
        }
        sdl3::event::Event::ControllerButtonUp { which, button, .. } => {
            let button = format!("{button:?}");
            (
                GamepadButtonUp::TOPIC,
                GamepadButtonUp {
                    id: *which,
                    button: &button,
                }
                .encode(),
            )
        }
        _ => return None,
    };
    Some(Envelope {
        topic: topic.to_string(),
        sender: sender.to_string(),
        correlation: None,
        payload,
    })
}

#[cfg(test)]
mod tests;
