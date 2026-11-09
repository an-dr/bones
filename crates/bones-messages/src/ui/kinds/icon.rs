use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// One Lucide icon, named as on lucide.dev (`"trash-2"`). Kind `"icon"`;
/// a click reports `Clicked` when the id is not zero. An unknown name draws
/// nothing. Rust owners can also put `ui::icons` glyphs in any text.
#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    /// The lucide.dev name.
    pub name: String,
    /// Height in points; `0` takes the body text size.
    pub size: f32,
}

impl WidgetProps for Icon {
    const KIND: &'static str = "icon";

    fn encode(&self) -> Vec<u8> {
        Writer::new().str(&self.name).f32(self.size).finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let icon = Self {
            name: reader.read_str()?.to_owned(),
            size: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(icon)
    }
}
