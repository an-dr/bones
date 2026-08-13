use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

macro_rules! gamepad_button_message {
    ($name:ident, $topic:literal, $description:literal) => {
        #[doc = $description]
        #[doc = ""]
        #[doc = "`button` is SDL's own button name (\"South\", \"DPadUp\", …)."]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name<'a> {
            pub id: u32,
            pub button: &'a str,
        }

        impl Message for $name<'_> {
            const TOPIC: &'static str = $topic;
        }

        impl EncodeMessage for $name<'_> {
            fn encode(&self) -> Vec<u8> {
                Writer::new().u32(self.id).str(self.button).finish()
            }
        }

        impl<'a> DecodeMessage<'a> for $name<'a> {
            fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
                let mut reader = Reader::new(payload);
                let message = Self {
                    id: reader.read_u32()?,
                    button: reader.read_str()?,
                };
                reader.finish()?;
                Ok(message)
            }
        }
    };
}

gamepad_button_message!(
    GamepadButtonDown,
    "input/gamepad-button-down",
    "A gamepad button was pressed."
);
gamepad_button_message!(
    GamepadButtonUp,
    "input/gamepad-button-up",
    "A gamepad button was released."
);
