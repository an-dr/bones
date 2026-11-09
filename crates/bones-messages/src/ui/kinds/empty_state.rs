use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// A centred icon, title, and detail where a screen has nothing to show.
/// Kind `"empty-state"`; no events.
#[derive(Debug, Clone, PartialEq)]
pub struct EmptyState {
    /// A lucide.dev icon name.
    pub icon: String,
    /// What is missing.
    pub title: String,
    /// What to do about it.
    pub detail: String,
}

impl WidgetProps for EmptyState {
    const KIND: &'static str = "empty-state";

    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .str(&self.icon)
            .str(&self.title)
            .str(&self.detail)
            .finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let props = Self {
            icon: reader.read_str()?.to_owned(),
            title: reader.read_str()?.to_owned(),
            detail: reader.read_str()?.to_owned(),
        };
        reader.finish()?;
        Ok(props)
    }
}
