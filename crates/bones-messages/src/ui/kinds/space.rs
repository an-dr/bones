use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// Empty room between neighbours. Kind `"space"`; no events.
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    /// The room, in points.
    pub amount: f32,
}

impl WidgetProps for Space {
    const KIND: &'static str = "space";

    fn encode(&self) -> Vec<u8> {
        Writer::new().f32(self.amount).finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let amount = reader.read_f32()?;
        reader.finish()?;
        Ok(Self { amount })
    }
}
