use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

macro_rules! mouse_button_message {
    ($name:ident, $topic:literal, $description:literal) => {
        #[doc = $description]
        #[doc = ""]
        #[doc = "`button` is SDL's own code: 1=left, 2=middle, 3=right, 4=x1, 5=x2, 0=unknown."]
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name {
            pub button: u8,
            pub x: f32,
            pub y: f32,
        }

        impl Message for $name {
            const TOPIC: &'static str = $topic;
        }

        impl EncodeMessage for $name {
            fn encode(&self) -> Vec<u8> {
                Writer::new()
                    .u8(self.button)
                    .f32(self.x)
                    .f32(self.y)
                    .finish()
            }
        }

        impl<'a> DecodeMessage<'a> for $name {
            fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
                let mut reader = Reader::new(payload);
                let message = Self {
                    button: reader.read_u8()?,
                    x: reader.read_f32()?,
                    y: reader.read_f32()?,
                };
                reader.finish()?;
                Ok(message)
            }
        }
    };
}

mouse_button_message!(MouseDown, "input/mouse-down", "A mouse button was pressed.");
mouse_button_message!(MouseUp, "input/mouse-up", "A mouse button was released.");
