//! Typed `audio/*` commands shared by extensions and the audio module.

mod command;
mod load_sound;
mod play_music;
mod play_sound;
mod set_music_volume;
mod stop_music;

pub use command::Command;
pub use load_sound::LoadSound;
pub use play_music::PlayMusic;
pub use play_sound::PlaySound;
pub use set_music_volume::SetMusicVolume;
pub use stop_music::StopMusic;

#[cfg(test)]
mod tests;
