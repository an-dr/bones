use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A short message floating at the bottom of the window, above everything.
/// Kind `"toast"`; its close button reports `Clicked`. The owner decides how
/// long it stays by showing it or not.
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    /// The message.
    pub text: String,
}

impl WidgetProps for Toast {
    const KIND: &'static str = "toast";

    fn encode(&self) -> Vec<u8> {
        Writer::new().str(&self.text).finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let props = Self {
            text: reader.read_str()?.to_owned(),
        };
        reader.finish()?;
        Ok(props)
    }
}
