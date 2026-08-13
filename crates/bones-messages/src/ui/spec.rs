use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use super::Widget;

/// A full per-frame panel (ADR-005): published every tick the owning
/// extension wants its UI visible. Not publishing this frame means nothing
/// is drawn this frame — there is no retained state to fall back on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec<'a> {
    pub title: &'a str,
    pub widgets: Vec<Widget<'a>>,
}

impl Message for Spec<'_> {
    const TOPIC: &'static str = "ui/spec";
}

impl EncodeMessage for Spec<'_> {
    fn encode(&self) -> Vec<u8> {
        let widget_count: u16 = self
            .widgets
            .len()
            .try_into()
            .expect("more than u16::MAX widgets in one spec");
        let mut writer = Writer::new().str(self.title).u16(widget_count);
        for widget in &self.widgets {
            writer = widget.encode_into(writer);
        }
        writer.finish()
    }
}

impl<'a> DecodeMessage<'a> for Spec<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let title = reader.read_str()?;
        let count = reader.read_u16()?;
        let mut widgets = Vec::with_capacity(count as usize);
        for _ in 0..count {
            widgets.push(Widget::decode(&mut reader)?);
        }
        reader.finish()?;
        Ok(Self { title, widgets })
    }
}
