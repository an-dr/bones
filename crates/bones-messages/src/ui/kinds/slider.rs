use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A value dragged between two bounds. Kind `"slider"`; a drag reports `ValueChanged`.
#[derive(Debug, Clone, PartialEq)]
pub struct Slider {
    /// The current value.
    pub value: f32,
    /// The lowest value.
    pub min: f32,
    /// The highest value.
    pub max: f32,
}

impl WidgetProps for Slider {
    const KIND: &'static str = "slider";

    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .f32(self.value)
            .f32(self.min)
            .f32(self.max)
            .finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let value = reader.read_f32()?;
        let min = reader.read_f32()?;
        let max = reader.read_f32()?;
        reader.finish()?;
        Ok(Self { value, min, max })
    }
}
