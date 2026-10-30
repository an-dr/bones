use bones_messages::desktop::Result as DesktopResult;
use bones_messages::game_core::{Collision, EntityTransform};
use bones_messages::input::{
    GamepadAxis, GamepadButtonDown, GamepadButtonUp, GamepadConnected, GamepadDisconnected,
    KeyDown, KeyUp, MouseDown, MouseMove, MouseUp, MouseWheel,
};
use bones_messages::instance::Launched;
use bones_messages::lifecycle::LifecycleEvent;
use bones_messages::os::{Reply as OsReply, Result as OsResult};
use bones_messages::renderer::{Captured, DisplayChanged, LogicalCanvas};
use bones_messages::tick::Tick;
use bones_messages::ui::{Changed, Clicked};
use bones_messages::upgrader::Result as UpgraderResult;
use bones_messages::web::{PageMessage, PanelClosed, PanelFailed, PanelOpened};
use bones_messages::window::CloseRequested;
use bones_messages::{DecodeError, DecodeMessage, Message};

/// A message delivered to `on_message`, decoded by its topic.
///
/// - One variant per core topic an extension can subscribe to and receive.
/// - `Other` carries everything else unchanged: an extension's own topics,
///   and a direct `send`, which arrives with an empty topic.
/// - Borrows from the payload, so decoding copies nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum Inbound<'a> {
    /// `core/tick`.
    Tick(Tick),
    /// `core/lifecycle`.
    Lifecycle(LifecycleEvent<'a>),
    /// `window/close-requested`.
    CloseRequested(CloseRequested),
    /// `input/key-down`.
    KeyDown(KeyDown<'a>),
    /// `input/key-up`.
    KeyUp(KeyUp<'a>),
    /// `input/mouse-down`.
    MouseDown(MouseDown),
    /// `input/mouse-up`.
    MouseUp(MouseUp),
    /// `input/mouse-move`.
    MouseMove(MouseMove),
    /// `input/mouse-wheel`.
    MouseWheel(MouseWheel),
    /// `input/gamepad-connected`.
    GamepadConnected(GamepadConnected),
    /// `input/gamepad-disconnected`.
    GamepadDisconnected(GamepadDisconnected),
    /// `input/gamepad-button-down`.
    GamepadButtonDown(GamepadButtonDown<'a>),
    /// `input/gamepad-button-up`.
    GamepadButtonUp(GamepadButtonUp<'a>),
    /// `input/gamepad-axis`.
    GamepadAxis(GamepadAxis<'a>),
    /// `ui/clicked`.
    Clicked(Clicked),
    /// `ui/changed`.
    Changed(Changed<'a>),
    /// `web/panel-opened`.
    PanelOpened(PanelOpened<'a>),
    /// `web/panel-closed`.
    PanelClosed(PanelClosed<'a>),
    /// `web/panel-failed`.
    PanelFailed(PanelFailed<'a>),
    /// `web/page-message`.
    PageMessage(PageMessage<'a>),
    /// `renderer/display-changed`.
    DisplayChanged(DisplayChanged),
    /// `renderer/logical-canvas`.
    LogicalCanvas(LogicalCanvas),
    /// `renderer/captured`.
    Captured(Captured<'a>),
    /// `game-core/entity-transform`.
    EntityTransform(EntityTransform),
    /// `game-core/collision`.
    Collision(Collision),
    /// `instance/launched`.
    Launched(Launched<'a>),
    /// `desktop/result`.
    DesktopResult(DesktopResult<'a>),
    /// `os/result`.
    OsResult(OsResult<'a>),
    /// Caller-aware OS completion on os/reply.
    OsReply(OsReply<'a>),
    /// `upgrader/result`.
    UpgraderResult(UpgraderResult<'a>),
    /// Any other topic, or a direct send (empty topic), as delivered.
    Other {
        /// The topic, empty for a direct send.
        topic: &'a str,
        /// The payload, undecoded.
        payload: &'a [u8],
    },
}

impl<'a> Inbound<'a> {
    /// Decodes `payload` by `topic`. A core topic whose payload is malformed
    /// is an error, so a version mismatch surfaces at the first message.
    pub fn decode(topic: &'a str, payload: &'a [u8]) -> Result<Self, DecodeError> {
        fn as_<'a, M: DecodeMessage<'a>>(
            payload: &'a [u8],
            wrap: fn(M) -> Inbound<'a>,
        ) -> Result<Inbound<'a>, DecodeError> {
            M::decode(payload).map(wrap)
        }
        match topic {
            Tick::TOPIC => as_(payload, Self::Tick),
            LifecycleEvent::TOPIC => as_(payload, Self::Lifecycle),
            CloseRequested::TOPIC => as_(payload, Self::CloseRequested),
            KeyDown::TOPIC => as_(payload, Self::KeyDown),
            KeyUp::TOPIC => as_(payload, Self::KeyUp),
            MouseDown::TOPIC => as_(payload, Self::MouseDown),
            MouseUp::TOPIC => as_(payload, Self::MouseUp),
            MouseMove::TOPIC => as_(payload, Self::MouseMove),
            MouseWheel::TOPIC => as_(payload, Self::MouseWheel),
            GamepadConnected::TOPIC => as_(payload, Self::GamepadConnected),
            GamepadDisconnected::TOPIC => as_(payload, Self::GamepadDisconnected),
            GamepadButtonDown::TOPIC => as_(payload, Self::GamepadButtonDown),
            GamepadButtonUp::TOPIC => as_(payload, Self::GamepadButtonUp),
            GamepadAxis::TOPIC => as_(payload, Self::GamepadAxis),
            Clicked::TOPIC => as_(payload, Self::Clicked),
            Changed::TOPIC => as_(payload, Self::Changed),
            PanelOpened::TOPIC => as_(payload, Self::PanelOpened),
            PanelClosed::TOPIC => as_(payload, Self::PanelClosed),
            PanelFailed::TOPIC => as_(payload, Self::PanelFailed),
            PageMessage::TOPIC => as_(payload, Self::PageMessage),
            DisplayChanged::TOPIC => as_(payload, Self::DisplayChanged),
            LogicalCanvas::TOPIC => as_(payload, Self::LogicalCanvas),
            Captured::TOPIC => as_(payload, Self::Captured),
            EntityTransform::TOPIC => as_(payload, Self::EntityTransform),
            Collision::TOPIC => as_(payload, Self::Collision),
            Launched::TOPIC => as_(payload, Self::Launched),
            DesktopResult::TOPIC => as_(payload, Self::DesktopResult),
            OsResult::TOPIC => as_(payload, Self::OsResult),
            OsReply::TOPIC => as_(payload, Self::OsReply),
            UpgraderResult::TOPIC => as_(payload, Self::UpgraderResult),
            _ => Ok(Self::Other { topic, payload }),
        }
    }
}

#[cfg(test)]
mod tests;
