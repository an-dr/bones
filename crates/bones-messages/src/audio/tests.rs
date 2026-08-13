use super::*;
use crate::{DecodeError, EncodeMessage, Message};

#[test]
fn non_audio_topics_are_ignored() {
    assert_eq!(Command::decode("input/key-down", b"whatever"), Ok(None));
}

#[test]
fn every_command_round_trips() {
    let load = LoadSound {
        id: 3,
        bytes: b"not-really-audio",
    };
    assert_eq!(
        Command::decode(LoadSound::TOPIC, &load.encode()),
        Ok(Some(Command::LoadSound(load)))
    );

    let play = PlaySound { id: 3, volume: 0.8 };
    assert_eq!(
        Command::decode(PlaySound::TOPIC, &play.encode()),
        Ok(Some(Command::PlaySound(play)))
    );

    let music = PlayMusic { id: 7, volume: 0.5 };
    assert_eq!(
        Command::decode(PlayMusic::TOPIC, &music.encode()),
        Ok(Some(Command::PlayMusic(music)))
    );

    let stop = StopMusic;
    assert_eq!(
        Command::decode(StopMusic::TOPIC, &stop.encode()),
        Ok(Some(Command::StopMusic(stop)))
    );

    let volume = SetMusicVolume { volume: 0.3 };
    assert_eq!(
        Command::decode(SetMusicVolume::TOPIC, &volume.encode()),
        Ok(Some(Command::SetMusicVolume(volume)))
    );
}

#[test]
fn fixed_shape_commands_reject_wrong_byte_counts() {
    assert_eq!(
        Command::decode(PlaySound::TOPIC, &[0; 7]),
        Err(DecodeError::Truncated)
    );
    assert_eq!(
        Command::decode(StopMusic::TOPIC, &[0; 1]),
        Err(DecodeError::TrailingBytes),
        "StopMusic reads no fields, so any nonzero payload is trailing, not truncated"
    );
}
