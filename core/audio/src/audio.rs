//! The `Audio` native module (design/modules.md, ADR-011, ADR-018): plays
//! sound effects and music via `audio/*` commands, backed by `kira`
//! (engine-agnostic, no window/App coupling — see ADR-019's
//! crate-sourcing rationale).

use std::collections::HashMap;
use std::io::Cursor;

use bones_messages::audio::Command;
use bus::{Envelope, Handler, Module, ModuleContext};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::Tween;

use crate::decibels::linear_to_decibels;

pub struct Audio {
    manager: Option<kira::AudioManager<kira::DefaultBackend>>,
    // Cached decoded audio, keyed by an application-assigned id — its own
    // namespace, separate from `gfx`'s sprite ids and `ui`'s texture ids.
    sounds: HashMap<u32, StaticSoundData>,
    // At most one active music track (tactical simplification, see
    // `bones_messages::audio::PlayMusic`) — `PlaySound` effects are
    // fire-and-forget and keep playing after their handle is dropped
    // (playback lives on the audio thread, not tied to the handle), so
    // only music needs a retained handle to stop/adjust later.
    music: Option<StaticSoundHandle>,
}

impl Audio {
    pub fn new() -> Self {
        Self {
            manager: None,
            sounds: HashMap::new(),
            music: None,
        }
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler for Audio {
    // No logger field (unlike `Renderer`):
    // - the generic `.module(...)` path that constructs this has no
    //   access to `Engine`'s internal logger — see the `TODO` on
    //   `Engine::module`.
    // - same limitation `core/platform` already accepts for a gamepad
    //   that fails to open.
    // - decode failures, an unknown sound id, or a backend not yet open
    //   (`Module::init` never ran or failed) are silently no-ops rather
    //   than reported nowhere useful.
    fn handle(&mut self, envelope: &Envelope) {
        let Some(command) = Command::decode(&envelope.topic, &envelope.payload)
            .ok()
            .flatten()
        else {
            return;
        };
        let Some(manager) = self.manager.as_mut() else {
            return;
        };
        match command {
            Command::LoadSound(load) => {
                if let Ok(data) = StaticSoundData::from_cursor(Cursor::new(load.bytes.to_vec())) {
                    self.sounds.insert(load.id, data);
                }
            }
            Command::PlaySound(play) => {
                if let Some(data) = self.sounds.get(&play.id) {
                    let data = data.clone().volume(linear_to_decibels(play.volume));
                    let _ = manager.play(data);
                }
            }
            Command::PlayMusic(play) => {
                if let Some(data) = self.sounds.get(&play.id) {
                    if let Some(mut previous) = self.music.take() {
                        previous.stop(Tween::default());
                    }
                    let data = data
                        .clone()
                        .loop_region(..)
                        .volume(linear_to_decibels(play.volume));
                    if let Ok(handle) = manager.play(data) {
                        self.music = Some(handle);
                    }
                }
            }
            Command::StopMusic(_) => {
                if let Some(mut handle) = self.music.take() {
                    handle.stop(Tween::default());
                }
            }
            Command::SetMusicVolume(set) => {
                if let Some(handle) = self.music.as_mut() {
                    handle.set_volume(linear_to_decibels(set.volume), Tween::default());
                }
            }
        }
    }
}

impl Module for Audio {
    fn name(&self) -> &str {
        "audio"
    }

    /// Opens the default output device.
    ///
    /// - Errors (e.g. no audio device present) fail the module.
    /// - Same stance as renderer's missing `window-surface`: a caller/
    ///   environment mistake, not a runtime condition to silently degrade
    ///   from.
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe("audio/*");
        let manager =
            kira::AudioManager::<kira::DefaultBackend>::new(kira::AudioManagerSettings::default())
                .map_err(|e| e.to_string())?;
        self.manager = Some(manager);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
