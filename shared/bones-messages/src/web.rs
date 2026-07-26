//! Typed web-panel commands and events (ADR-006).
//!
//! Commands are direct requests to [`ENDPOINT`], so a guest can detect a
//! build without the optional web module as `UnknownEndpoint`. Events use
//! `web/*` topics and carry both the host-derived owner and its local panel
//! id because pub/sub delivery is broadcast.

mod close_panel;
mod command;
mod navigate;
mod open_panel;
mod page_message;
mod panel_closed;
mod panel_failed;
mod panel_opened;
mod panel_source;
mod send_json;

pub use close_panel::ClosePanel;
pub use command::Command;
pub use navigate::Navigate;
pub use open_panel::OpenPanel;
pub use page_message::PageMessage;
pub use panel_closed::PanelClosed;
pub use panel_failed::PanelFailed;
pub use panel_opened::PanelOpened;
pub use panel_source::PanelSource;
pub use send_json::SendJson;

/// Direct-send endpoint registered by the optional native web module.
pub const ENDPOINT: &str = "web";

#[cfg(test)]
mod tests;
