//! Extension lifecycle transitions published on `core/lifecycle`.

use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// A transition in an extension's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Loaded,
    Faulted,
    Reloading,
    Reloaded,
    Stopped,
}

impl Event {
    fn tag(self) -> u8 {
        match self {
            Self::Loaded => 0,
            Self::Faulted => 1,
            Self::Reloading => 2,
            Self::Reloaded => 3,
            Self::Stopped => 4,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            0 => Ok(Self::Loaded),
            1 => Ok(Self::Faulted),
            2 => Ok(Self::Reloading),
            3 => Ok(Self::Reloaded),
            4 => Ok(Self::Stopped),
            tag => Err(DecodeError::InvalidTag {
                message: "lifecycle event",
                tag,
            }),
        }
    }
}

/// A lifecycle transition and the extension it concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleEvent<'a> {
    /// The transition that occurred.
    pub event: Event,
    /// Extension name borrowed from the decoded payload when possible.
    pub extension: &'a str,
}

impl Message for LifecycleEvent<'_> {
    const TOPIC: &'static str = "core/lifecycle";
}

impl EncodeMessage for LifecycleEvent<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u8(self.event.tag())
            .bytes(self.extension.as_bytes())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for LifecycleEvent<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let event = Event::from_tag(reader.read_u8()?)?;
        let extension = reader.read_str_rest()?;
        Ok(Self { event, extension })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_round_trips() {
        for event in [
            Event::Loaded,
            Event::Faulted,
            Event::Reloading,
            Event::Reloaded,
            Event::Stopped,
        ] {
            let message = LifecycleEvent {
                event,
                extension: "level",
            };
            assert_eq!(LifecycleEvent::decode(&message.encode()), Ok(message));
        }
    }

    #[test]
    fn invalid_event_and_name_are_structured_errors() {
        assert_eq!(
            LifecycleEvent::decode(&[255, b'x']),
            Err(DecodeError::InvalidTag {
                message: "lifecycle event",
                tag: 255
            })
        );
        assert_eq!(
            LifecycleEvent::decode(&[0, 0xff]),
            Err(DecodeError::InvalidUtf8)
        );
    }
}
