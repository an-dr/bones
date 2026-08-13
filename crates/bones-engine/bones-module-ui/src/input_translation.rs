pub(crate) fn translate_mouse_button(
    button: sdl3::mouse::MouseButton,
) -> Option<egui::PointerButton> {
    match button {
        sdl3::mouse::MouseButton::Left => Some(egui::PointerButton::Primary),
        sdl3::mouse::MouseButton::Right => Some(egui::PointerButton::Secondary),
        sdl3::mouse::MouseButton::Middle => Some(egui::PointerButton::Middle),
        _ => None,
    }
}

/// Only the handful of keys a single-line text field needs to navigate and
/// edit; egui receives typed characters separately via `Event::Text`.
pub(crate) fn translate_key(keycode: sdl3::keyboard::Keycode) -> Option<egui::Key> {
    use sdl3::keyboard::Keycode as K;
    match keycode {
        K::Backspace => Some(egui::Key::Backspace),
        K::Delete => Some(egui::Key::Delete),
        K::Return | K::Return2 | K::KpEnter => Some(egui::Key::Enter),
        K::Escape => Some(egui::Key::Escape),
        K::Tab => Some(egui::Key::Tab),
        K::Left => Some(egui::Key::ArrowLeft),
        K::Right => Some(egui::Key::ArrowRight),
        K::Up => Some(egui::Key::ArrowUp),
        K::Down => Some(egui::Key::ArrowDown),
        K::Home => Some(egui::Key::Home),
        K::End => Some(egui::Key::End),
        _ => None,
    }
}

pub(crate) fn compute_modifiers(keymod: sdl3::keyboard::Mod) -> egui::Modifiers {
    use sdl3::keyboard::Mod;
    let ctrl = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
    egui::Modifiers {
        alt: keymod.intersects(Mod::LALTMOD | Mod::RALTMOD),
        ctrl,
        shift: keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
        mac_cmd: false,
        command: ctrl,
    }
}
