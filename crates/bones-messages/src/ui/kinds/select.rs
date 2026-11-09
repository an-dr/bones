use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A drop-down choice of one option. Kind `"select"`; choosing reports `Selected` with the option's index.
#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    /// The index of the chosen option, if any.
    pub selected: Option<u32>,
    /// Shown when nothing is chosen.
    pub placeholder: String,
    /// The options, in order.
    pub options: Vec<String>,
}

impl WidgetProps for Select {
    const KIND: &'static str = "select";

    fn encode(&self) -> Vec<u8> {
        let count: u16 = self
            .options
            .len()
            .try_into()
            .expect("more than u16::MAX options");
        let mut writer = Writer::new()
            .u32(self.selected.unwrap_or(u32::MAX))
            .str(&self.placeholder)
            .u16(count);
        for option in &self.options {
            writer = writer.str(option);
        }
        writer.finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let selected = Some(reader.read_u32()?).filter(|index| *index != u32::MAX);
        let placeholder = reader.read_str()?.to_owned();
        let count = reader.read_u16()?;
        let mut options = Vec::with_capacity(count as usize);
        for _ in 0..count {
            options.push(reader.read_str()?.to_owned());
        }
        reader.finish()?;
        Ok(Self {
            selected,
            placeholder,
            options,
        })
    }
}
