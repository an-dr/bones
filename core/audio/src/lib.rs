//! Audio module (design/modules.md, ADR-011, ADR-018): plays sound effects
//! and music via `audio/*` commands, backed by `kira` (engine-agnostic, no
//! window/App coupling — see ADR-019's crate-sourcing rationale).

use std::collections::HashMap;
use std::io::Cursor;

use bones_messages::audio::Command;
use bus::{Envelope, Handler, Module, ModuleContext};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{Decibels, Tween};

/// Converts a linear amplitude (`0.0` silent, `1.0` unity gain — the wire
/// convention every `audio/*` volume field uses) into kira's own decibel
/// scale. `Decibels` has `as_amplitude` for the reverse direction but no
/// public constructor the other way, hence this: the standard
/// amplitude-to-dB formula, floored at `Decibels::SILENCE` so a `volume`
/// of exactly `0.0` (or anything non-positive) doesn't compute `-inf`.
fn linear_to_decibels(volume: f32) -> Decibels {
    if volume <= 0.0 {
        Decibels::SILENCE
    } else {
        Decibels((20.0 * volume.log10()).max(Decibels::SILENCE.0))
    }
}

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
    // No logger field (unlike `Renderer`): the generic `.module(...)` path
    // that constructs this doesn't have access to `Engine`'s internal
    // logger (only `.renderer()`/`.ui()`'s hardcoded, privileged wiring
    // does) — same limitation `core/platform` already accepts for a
    // gamepad that fails to open. Decode failures, an unknown sound id, or
    // a backend not yet open (`Module::init` never ran or failed) are
    // silently no-ops rather than reported nowhere useful.
    fn handle(&mut self, envelope: &Envelope) {
        let Some(command) = Command::decode(&envelope.topic, &envelope.payload).ok().flatten() else {
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
                    let data = data.clone().loop_region(..).volume(linear_to_decibels(play.volume));
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

    /// Opens the default output device. Errors (e.g. no audio device
    /// present) fail the module the same way renderer's missing
    /// `window-surface` does — a caller/environment mistake, not a runtime
    /// condition to silently degrade from.
    fn init(&mut self, ctx: &mut ModuleContext) -> Result<(), String> {
        ctx.subscribe("audio/*");
        let manager = kira::AudioManager::<kira::DefaultBackend>::new(kira::AudioManagerSettings::default())
            .map_err(|e| e.to_string())?;
        self.manager = Some(manager);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bones_messages::{EncodeMessage, Message};
    use bus::ServiceRegistry;

    #[test]
    fn init_opens_a_real_audio_device_and_subscribes_audio_topics() {
        let mut registry = ServiceRegistry::new();
        let mut ctx = ModuleContext::new(&mut registry);
        let mut audio = Audio::new();

        audio.init(&mut ctx).expect("a real audio device should open on this machine");

        assert_eq!(ctx.into_subscriptions(), vec!["audio/*"]);
        assert!(audio.manager.is_some());
    }

    #[test]
    fn name_is_audio() {
        assert_eq!(Audio::new().name(), "audio");
    }

    /// A minimal valid WAV file (RIFF/WAVE, 16-bit PCM mono, a handful of
    /// silent samples) — small enough to inline, real enough to exercise
    /// `StaticSoundData::from_cursor`'s actual symphonia decode path,
    /// avoiding a binary test asset.
    fn tiny_wav() -> Vec<u8> {
        let samples: [i16; 8] = [0; 8];
        let data_bytes = samples.len() * 2;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&44100u32.to_le_bytes()); // sample rate
        wav.extend_from_slice(&(44100u32 * 2).to_le_bytes()); // byte rate
        wav.extend_from_slice(&2u16.to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data_bytes as u32).to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        wav
    }

    fn ready_audio() -> Audio {
        let mut registry = ServiceRegistry::new();
        let mut ctx = ModuleContext::new(&mut registry);
        let mut audio = Audio::new();
        audio.init(&mut ctx).expect("a real audio device should open on this machine");
        audio
    }

    fn envelope(topic: &str, payload: Vec<u8>) -> Envelope {
        Envelope {
            topic: topic.to_string(),
            sender: "test".to_string(),
            correlation: None,
            payload,
        }
    }

    #[test]
    fn load_sound_decodes_a_real_wav_into_the_cache() {
        let mut audio = ready_audio();
        let load = bones_messages::audio::LoadSound { id: 1, bytes: &tiny_wav() };
        audio.handle(&envelope(bones_messages::audio::LoadSound::TOPIC, load.encode()));

        assert!(audio.sounds.contains_key(&1));
    }

    #[test]
    fn play_music_retains_a_playing_handle_and_stop_music_clears_it() {
        let mut audio = ready_audio();
        let load = bones_messages::audio::LoadSound { id: 1, bytes: &tiny_wav() };
        audio.handle(&envelope(bones_messages::audio::LoadSound::TOPIC, load.encode()));

        let play = bones_messages::audio::PlayMusic { id: 1, volume: 0.5 };
        audio.handle(&envelope(bones_messages::audio::PlayMusic::TOPIC, play.encode()));

        let handle = audio.music.as_ref().expect("play-music should retain a handle");
        assert_eq!(handle.state(), kira::sound::PlaybackState::Playing);

        audio.handle(&envelope(bones_messages::audio::StopMusic::TOPIC, bones_messages::audio::StopMusic.encode()));
        assert!(audio.music.is_none(), "stop-music should clear the retained handle");
    }

    #[test]
    fn play_sound_and_set_music_volume_for_an_unloaded_or_absent_id_do_not_panic() {
        let mut audio = ready_audio();
        // No LoadSound first — an extension racing its own load/play order,
        // or a bogus id, must not crash the render loop.
        let play = bones_messages::audio::PlaySound { id: 99, volume: 1.0 };
        audio.handle(&envelope(bones_messages::audio::PlaySound::TOPIC, play.encode()));

        let set_volume = bones_messages::audio::SetMusicVolume { volume: 0.2 };
        audio.handle(&envelope(bones_messages::audio::SetMusicVolume::TOPIC, set_volume.encode()));
        // No music ever started — nothing to adjust, and no panic either.
        assert!(audio.music.is_none());
    }

    #[test]
    fn malformed_and_unknown_payloads_are_silently_ignored() {
        let mut audio = ready_audio();
        audio.handle(&envelope("audio/play-sound", vec![1, 2, 3]));
        audio.handle(&envelope("audio/does-not-exist", vec![]));
        // Reaching here without panicking is the assertion.
    }
}
