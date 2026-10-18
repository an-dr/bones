use super::*;
use bones_kernel_infra::bus::ServiceRegistry;
use bones_messages::{EncodeMessage, Message};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

/// One test at a time touches the audio device. libtest runs tests in
/// parallel, and on a machine without a device concurrent failing opens
/// crash the Windows backend (an access violation, not an error).
static DEVICE: Mutex<()> = Mutex::new(());

/// Set once a test finds no output device, so the rest skip without
/// opening it again.
static NO_DEVICE: OnceLock<()> = OnceLock::new();

fn lock_device() -> MutexGuard<'static, ()> {
    DEVICE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs `init`, or returns `None` when the machine has no audio output
/// device -- a headless CI runner, for one -- after reporting the skip. Any
/// other failure still fails the test.
fn init_or_skip(audio: &mut Audio, ctx: &mut ModuleContext) -> Option<()> {
    if NO_DEVICE.get().is_some() {
        eprintln!("skipped: this machine has no audio output device");
        return None;
    }
    match audio.init(ctx) {
        Ok(()) => Some(()),
        Err(err) if is_no_audio_device(&err) => {
            eprintln!("skipped: this machine has no audio output device ({err})");
            let _ = NO_DEVICE.set(());
            None
        }
        Err(err) => panic!("a real audio device should open on this machine: {err}"),
    }
}

/// Whether an `init` error means the machine has no usable output device:
/// cpal words it differently per platform, so headless Windows and Linux
/// runners each produce one of these.
fn is_no_audio_device(error: &str) -> bool {
    [
        "Cannot find the default audio output device",
        "The requested device is no longer available",
    ]
    .iter()
    .any(|phrase| error.contains(phrase))
}

#[test]
fn init_opens_a_real_audio_device_and_subscribes_audio_topics() {
    let _device = lock_device();
    let mut registry = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut audio = Audio::new();

    if init_or_skip(&mut audio, &mut ctx).is_none() {
        return;
    }

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

fn ready_audio() -> Option<Audio> {
    let mut registry = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut audio = Audio::new();
    init_or_skip(&mut audio, &mut ctx)?;
    Some(audio)
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
    let _device = lock_device();
    let Some(mut audio) = ready_audio() else {
        return;
    };
    let load = bones_messages::audio::LoadSound {
        id: 1,
        bytes: &tiny_wav(),
    };
    audio.handle(&envelope(
        bones_messages::audio::LoadSound::TOPIC,
        load.encode(),
    ));

    assert!(audio.sounds.contains_key(&1));
}

#[test]
fn play_music_retains_a_playing_handle_and_stop_music_clears_it() {
    let _device = lock_device();
    let Some(mut audio) = ready_audio() else {
        return;
    };
    let load = bones_messages::audio::LoadSound {
        id: 1,
        bytes: &tiny_wav(),
    };
    audio.handle(&envelope(
        bones_messages::audio::LoadSound::TOPIC,
        load.encode(),
    ));

    let play = bones_messages::audio::PlayMusic { id: 1, volume: 0.5 };
    audio.handle(&envelope(
        bones_messages::audio::PlayMusic::TOPIC,
        play.encode(),
    ));

    let handle = audio
        .music
        .as_ref()
        .expect("play-music should retain a handle");
    assert_eq!(handle.state(), kira::sound::PlaybackState::Playing);

    audio.handle(&envelope(
        bones_messages::audio::StopMusic::TOPIC,
        bones_messages::audio::StopMusic.encode(),
    ));
    assert!(
        audio.music.is_none(),
        "stop-music should clear the retained handle"
    );
}

#[test]
fn play_sound_and_set_music_volume_for_an_unloaded_or_absent_id_do_not_panic() {
    let _device = lock_device();
    let Some(mut audio) = ready_audio() else {
        return;
    };
    // No LoadSound first — an extension racing its own load/play order,
    // or a bogus id, must not crash the render loop.
    let play = bones_messages::audio::PlaySound {
        id: 99,
        volume: 1.0,
    };
    audio.handle(&envelope(
        bones_messages::audio::PlaySound::TOPIC,
        play.encode(),
    ));

    let set_volume = bones_messages::audio::SetMusicVolume { volume: 0.2 };
    audio.handle(&envelope(
        bones_messages::audio::SetMusicVolume::TOPIC,
        set_volume.encode(),
    ));
    // No music ever started — nothing to adjust, and no panic either.
    assert!(audio.music.is_none());
}

#[test]
fn malformed_and_unknown_payloads_are_silently_ignored() {
    let _device = lock_device();
    let Some(mut audio) = ready_audio() else {
        return;
    };
    audio.handle(&envelope("audio/play-sound", vec![1, 2, 3]));
    audio.handle(&envelope("audio/does-not-exist", vec![]));
    // Reaching here without panicking is the assertion.
}
