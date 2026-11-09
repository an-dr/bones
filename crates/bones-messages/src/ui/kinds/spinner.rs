use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A busy indicator. Kind `"spinner"`; no events.
#[derive(Debug, Clone, PartialEq)]
pub struct Spinner;

impl WidgetProps for Spinner {
    const KIND: &'static str = "spinner";

    fn encode(&self) -> Vec<u8> {
        Writer::new().finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let reader = Reader::new(props);
        reader.finish()?;
        Ok(Self)
    }
}
