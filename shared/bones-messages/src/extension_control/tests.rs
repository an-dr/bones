use super::*;
use crate::{DecodeError, DecodeMessage, EncodeMessage, Message};

#[test]
fn every_command_round_trips_and_dispatches() {
    let load = Load {
        extension: "level_one",
    };
    assert_eq!(Load::decode(&load.encode()), Ok(load));
    assert_eq!(
        Command::decode(Load::TOPIC, &load.encode()),
        Ok(Some(Command::Load(load)))
    );

    let unload = Unload { extension: "will" };
    assert_eq!(Unload::decode(&unload.encode()), Ok(unload));
    assert_eq!(
        Command::decode(Unload::TOPIC, &unload.encode()),
        Ok(Some(Command::Unload(unload)))
    );

    let reload = Reload { extension: "menu" };
    assert_eq!(Reload::decode(&reload.encode()), Ok(reload));
    assert_eq!(
        Command::decode(Reload::TOPIC, &reload.encode()),
        Ok(Some(Command::Reload(reload)))
    );
}

#[test]
fn malformed_names_and_unknown_topics_are_rejected_cleanly() {
    assert_eq!(Load::decode(&[0xff]), Err(DecodeError::InvalidUtf8));
    assert_eq!(Command::decode("core/extensions/nope", &[]), Ok(None));
}
