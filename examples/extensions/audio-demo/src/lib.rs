use bones_wasm_sdk::Guest;
use bones_wasm_sdk::bindings::bones::core::host_api::{log, publish, subscribe, Level};
use bones_wasm_sdk::messages::audio::{LoadSound, PlayMusic, PlaySound};
use bones_wasm_sdk::messages::input::KeyDown;
use bones_wasm_sdk::messages::{EncodeMessage, Message};

const SFX_ID: u32 = 1;
const MUSIC_ID: u32 = 2;
const SAMPLE_RATE: u32 = 44_100;

/// Synthesizes a mono 16-bit PCM WAV sine tone over an exact whole number
/// of `cycles` of `frequency_hz` — the waveform then starts and ends at a
/// zero crossing, so a one-shot beep has no end-of-buffer click and a
/// looping track has no seam, with no extra fade logic needed either way.
/// Avoids embedding a licensed audio asset just for a demo.
fn synthesize_tone(frequency_hz: f32, cycles: u32, amplitude: f32) -> Vec<u8> {
    let sample_count = ((cycles as f32 / frequency_hz) * SAMPLE_RATE as f32).round() as u32;
    let mut samples = Vec::with_capacity(sample_count as usize);
    for n in 0..sample_count {
        let t = n as f32 / SAMPLE_RATE as f32;
        let value = (amplitude * (2.0 * core::f32::consts::PI * frequency_hz * t).sin() * i16::MAX as f32) as i16;
        samples.push(value);
    }

    let data_bytes = samples.len() * 2;
    let mut wav = Vec::with_capacity(44 + data_bytes);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

struct Component;

impl Guest for Component {
    fn shutdown() {}

    fn init() {
        subscribe(KeyDown::TOPIC);

        // ~150ms, 880Hz — a short click/beep for the one-shot SFX.
        let sfx = synthesize_tone(880.0, 132, 0.6);
        publish(LoadSound::TOPIC, &LoadSound { id: SFX_ID, bytes: &sfx }.encode());

        // 2s, 220Hz — a quiet, seamlessly-looping background tone.
        let music = synthesize_tone(220.0, 440, 0.15);
        publish(LoadSound::TOPIC, &LoadSound { id: MUSIC_ID, bytes: &music }.encode());
        publish(PlayMusic::TOPIC, &PlayMusic { id: MUSIC_ID, volume: 1.0 }.encode());

        log(Level::Info, "init: loaded sfx + music, press any key for the sfx");
    }

    fn on_tick(_dt: f32) {}

    fn on_message(topic: String, _sender: String, _payload: Vec<u8>) -> Option<Vec<u8>> {
        if topic == KeyDown::TOPIC {
            publish(PlaySound::TOPIC, &PlaySound { id: SFX_ID, volume: 0.8 }.encode());
        }
        None
    }
}

bones_wasm_sdk::export!(Component);
