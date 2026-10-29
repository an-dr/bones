//! Every core topic in one place, for checking a payload against its
//! contract without knowing its type in advance.
//!
//! - [`TOPICS`] lists the topics `wit/wire-format.json` specifies; a test keeps
//!   the two equal.
//! - [`validate`] decodes a payload by topic and discards the result, which
//!   is what a host does to report a malformed payload at its source.
//! - [`explain`] turns a failure into a line a person or an agent can act on.
//! - [`closest`] names the core topic a misspelt one most likely meant.

use crate::{audio, desktop, extension_control, game_core, gfx, input, instance, lifecycle, os};
use crate::{persistence, renderer, tick, ui, upgrader, web, window};
use crate::{DecodeError, DecodeMessage, Message};

/// Every core topic, in the order `wit/wire-format.md` specifies them.
pub const TOPICS: &[&str] = &[
    tick::Tick::TOPIC,
    lifecycle::LifecycleEvent::TOPIC,
    extension_control::Load::TOPIC,
    extension_control::Unload::TOPIC,
    extension_control::Reload::TOPIC,
    window::CloseRequested::TOPIC,
    input::KeyDown::TOPIC,
    input::KeyUp::TOPIC,
    input::MouseDown::TOPIC,
    input::MouseUp::TOPIC,
    input::MouseMove::TOPIC,
    input::MouseWheel::TOPIC,
    input::GamepadConnected::TOPIC,
    input::GamepadDisconnected::TOPIC,
    input::GamepadButtonDown::TOPIC,
    input::GamepadButtonUp::TOPIC,
    input::GamepadAxis::TOPIC,
    gfx::Clear::TOPIC,
    gfx::ClearDrawBatch::TOPIC,
    gfx::LoadSprite::TOPIC,
    gfx::DrawSprite::TOPIC,
    gfx::DrawRect::TOPIC,
    gfx::DrawLine::TOPIC,
    gfx::DrawCircle::TOPIC,
    gfx::DrawTriangle::TOPIC,
    gfx::DrawText::TOPIC,
    gfx::SetCamera::TOPIC,
    gfx::SetDisplay::TOPIC,
    renderer::DisplayChanged::TOPIC,
    renderer::LogicalCanvas::TOPIC,
    renderer::Capture::TOPIC,
    renderer::Captured::TOPIC,
    ui::Spec::TOPIC,
    ui::Clicked::TOPIC,
    ui::Changed::TOPIC,
    audio::LoadSound::TOPIC,
    audio::PlaySound::TOPIC,
    audio::PlayMusic::TOPIC,
    audio::SetMusicVolume::TOPIC,
    audio::StopMusic::TOPIC,
    game_core::EntityOpMessage::TOPIC,
    game_core::EntityTransform::TOPIC,
    game_core::Collision::TOPIC,
    game_core::LoadTilemap::TOPIC,
    web::PanelOpened::TOPIC,
    web::PanelClosed::TOPIC,
    web::PanelFailed::TOPIC,
    web::PageMessage::TOPIC,
    instance::Launched::TOPIC,
    desktop::Request::TOPIC,
    desktop::Result::TOPIC,
    os::Request::TOPIC,
    os::Result::TOPIC,
    upgrader::Request::TOPIC,
    upgrader::Result::TOPIC,
    persistence::Save::TOPIC,
];

/// Decodes `payload` as the core message `topic` names.
///
/// `None` when `topic` is not a core topic, so the bus's opaque application
/// traffic is never judged.
pub fn validate(topic: &str, payload: &[u8]) -> Option<Result<(), DecodeError>> {
    fn check<'a, M: DecodeMessage<'a>>(payload: &'a [u8]) -> Result<(), DecodeError> {
        M::decode(payload).map(|_| ())
    }
    fn found<T>(decoded: Result<Option<T>, DecodeError>) -> Option<Result<(), DecodeError>> {
        match decoded {
            Ok(Some(_)) => Some(Ok(())),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
    if let Some(result) = found(gfx::Command::decode(topic, payload)) {
        return Some(result);
    }
    if let Some(result) = found(audio::Command::decode(topic, payload)) {
        return Some(result);
    }
    if let Some(result) = found(extension_control::Command::decode(topic, payload)) {
        return Some(result);
    }
    let result = match topic {
        tick::Tick::TOPIC => check::<tick::Tick>(payload),
        lifecycle::LifecycleEvent::TOPIC => check::<lifecycle::LifecycleEvent>(payload),
        window::CloseRequested::TOPIC => check::<window::CloseRequested>(payload),
        input::KeyDown::TOPIC => check::<input::KeyDown>(payload),
        input::KeyUp::TOPIC => check::<input::KeyUp>(payload),
        input::MouseDown::TOPIC => check::<input::MouseDown>(payload),
        input::MouseUp::TOPIC => check::<input::MouseUp>(payload),
        input::MouseMove::TOPIC => check::<input::MouseMove>(payload),
        input::MouseWheel::TOPIC => check::<input::MouseWheel>(payload),
        input::GamepadConnected::TOPIC => check::<input::GamepadConnected>(payload),
        input::GamepadDisconnected::TOPIC => check::<input::GamepadDisconnected>(payload),
        input::GamepadButtonDown::TOPIC => check::<input::GamepadButtonDown>(payload),
        input::GamepadButtonUp::TOPIC => check::<input::GamepadButtonUp>(payload),
        input::GamepadAxis::TOPIC => check::<input::GamepadAxis>(payload),
        renderer::DisplayChanged::TOPIC => check::<renderer::DisplayChanged>(payload),
        renderer::LogicalCanvas::TOPIC => check::<renderer::LogicalCanvas>(payload),
        renderer::Capture::TOPIC => check::<renderer::Capture>(payload),
        renderer::Captured::TOPIC => check::<renderer::Captured>(payload),
        ui::Spec::TOPIC => check::<ui::Spec>(payload),
        ui::Clicked::TOPIC => check::<ui::Clicked>(payload),
        ui::Changed::TOPIC => check::<ui::Changed>(payload),
        game_core::EntityOpMessage::TOPIC => check::<game_core::EntityOpMessage>(payload),
        game_core::EntityTransform::TOPIC => check::<game_core::EntityTransform>(payload),
        game_core::Collision::TOPIC => check::<game_core::Collision>(payload),
        game_core::LoadTilemap::TOPIC => check::<game_core::LoadTilemap>(payload),
        web::PanelOpened::TOPIC => check::<web::PanelOpened>(payload),
        web::PanelClosed::TOPIC => check::<web::PanelClosed>(payload),
        web::PanelFailed::TOPIC => check::<web::PanelFailed>(payload),
        web::PageMessage::TOPIC => check::<web::PageMessage>(payload),
        instance::Launched::TOPIC => check::<instance::Launched>(payload),
        desktop::Request::TOPIC => check::<desktop::Request>(payload),
        desktop::Result::TOPIC => check::<desktop::Result>(payload),
        os::Request::TOPIC => check::<os::Request>(payload),
        os::Result::TOPIC => check::<os::Result>(payload),
        upgrader::Request::TOPIC => check::<upgrader::Request>(payload),
        upgrader::Result::TOPIC => check::<upgrader::Result>(payload),
        persistence::Save::TOPIC => check::<persistence::Save>(payload),
        _ => return None,
    };
    Some(result)
}

/// The core topic within two edits of `topic`, when `topic` is not one
/// itself: what a misspelt topic most likely meant.
pub fn closest(topic: &str) -> Option<&'static str> {
    if TOPICS.contains(&topic) {
        return None;
    }
    TOPICS
        .iter()
        .map(|candidate| (edit_distance(topic, candidate), *candidate))
        .filter(|(distance, _)| *distance <= MAX_EDITS)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

/// How far a topic may be from a core one and still count as a misspelling.
const MAX_EDITS: usize = 2;

/// Levenshtein distance over characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != *cb);
            current.push(substitution.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous[b.len()]
}
/// How many leading payload bytes [`explain`] shows.
const PREVIEW_BYTES: usize = 16;

/// A one-line account of why `payload` failed to decode as `topic`: the
/// failure, the payload's length and leading bytes, and where its layout is
/// specified.
pub fn explain(topic: &str, payload: &[u8], error: &DecodeError) -> String {
    let preview: String = payload
        .iter()
        .take(PREVIEW_BYTES)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let more = if payload.len() > PREVIEW_BYTES {
        "…"
    } else {
        ""
    };
    format!(
        "'{topic}' payload of {} bytes does not decode: {error} (bytes: {preview}{more}); \
         its layout is in wit/wire-format.md under `{topic}`",
        payload.len()
    )
}

#[cfg(test)]
mod tests;
