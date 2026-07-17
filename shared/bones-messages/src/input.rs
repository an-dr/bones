//! Keyboard input messages published by the platform.

use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

macro_rules! key_message {
    ($name:ident, $topic:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name<'a> {
            /// Platform key name borrowed from the decoded payload when possible.
            pub key: &'a str,
        }

        impl Message for $name<'_> {
            const TOPIC: &'static str = $topic;
        }

        impl EncodeMessage for $name<'_> {
            fn encode(&self) -> Vec<u8> {
                Writer::new().bytes(self.key.as_bytes()).finish()
            }
        }

        impl<'a> DecodeMessage<'a> for $name<'a> {
            fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
                let mut reader = Reader::new(payload);
                Ok(Self {
                    key: reader.read_str_rest()?,
                })
            }
        }
    };
}

key_message!(KeyDown, "input/key-down", "A keyboard key was pressed.");
key_message!(KeyUp, "input/key-up", "A keyboard key was released.");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_messages_round_trip_without_changing_the_wire_format() {
        let down = KeyDown { key: "Space" };
        assert_eq!(down.encode(), b"Space");
        assert_eq!(KeyDown::decode(&down.encode()), Ok(down));

        let up = KeyUp { key: "Escape" };
        assert_eq!(KeyUp::decode(&up.encode()), Ok(up));
    }
}
