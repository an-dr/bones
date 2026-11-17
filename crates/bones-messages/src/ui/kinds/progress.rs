use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A progress bar with an optional caption. Kind `"progress"`; no events.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// Done, from 0 to 1; negative for work of unknown length, drawn as a
    /// sweep; NaN for idle, which draws nothing but keeps the room. It is the
    /// same height in every state, so keep it in place while idle rather than
    /// removing it, and what is below it never moves.
    pub fraction: f32,
    /// Caption under the bar; empty for none.
    pub text: String,
}

impl WidgetProps for Progress {
    const KIND: &'static str = "progress";

    fn encode(&self) -> Vec<u8> {
        Writer::new().f32(self.fraction).str(&self.text).finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let props = Self {
            fraction: reader.read_f32()?,
            text: reader.read_str()?.to_owned(),
        };
        reader.finish()?;
        Ok(props)
    }
}
