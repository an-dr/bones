use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// The window's own title bar, drawn instead of the system frame: title,
/// optional icon, and minimise, maximise, and close buttons. Kind
/// `"title-bar"`; no events, the buttons act on the window themselves.
/// Place it first in a `Screen`'s top panel. The first time one is drawn the
/// ui module removes the system frame; where the platform refuses, the
/// system frame stays and this draws nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleBar {
    /// The window title shown in the bar.
    pub title: String,
    /// Handle of an uploaded image shown before the title; one with no image shows none.
    pub icon: u32,
}

impl WidgetProps for TitleBar {
    const KIND: &'static str = "title-bar";

    fn encode(&self) -> Vec<u8> {
        Writer::new().str(&self.title).u32(self.icon).finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let props = Self {
            title: reader.read_str()?.to_owned(),
            icon: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(props)
    }
}
