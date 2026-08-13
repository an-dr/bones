//! Theme-free layout, interaction, and gfx command helpers for game UIs.
//!
//! Absorbed from the former `game-ui` crate (ADR-025). It stays optional
//! behind the `game-ui` feature: a game chooses its own colors and flow while
//! reusing the geometry and input behaviour, and an extension that renders no
//! in-world UI never compiles it.

mod button;
mod button_layout;
mod canvas;
mod draw_command;
mod menu_layout;
mod rect;
mod selection;
mod vertical_menu;

pub use button::Button;
pub use button_layout::ButtonLayout;
pub use canvas::Canvas;
pub use draw_command::DrawCommand;
pub use menu_layout::MenuLayout;
pub use rect::Rect;
pub use selection::Selection;
pub use vertical_menu::VerticalMenu;

#[cfg(test)]
mod tests;
