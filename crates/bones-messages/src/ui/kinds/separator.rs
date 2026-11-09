use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A thin line between groups. Kind `"separator"`; no events.
#[derive(Debug, Clone, PartialEq)]
pub struct Separator;

impl WidgetProps for Separator {
    const KIND: &'static str = "separator";

    fn encode(&self) -> Vec<u8> {
        Writer::new().finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let reader = Reader::new(props);
        reader.finish()?;
        Ok(Self)
    }
}
