//! Typed `ui/*` widget-spec and interaction messages (ADR-005, design/
//! presentation.md): the immediate-mode vocabulary extensions use to
//! describe egui panels, and the events the ui module publishes back. The
//! vocabulary is deliberately small at first (`Label`, `TextEdit`,
//! `Button`) — enough for the "notes" worked example; grows as a versioned
//! addition, same as `gfx`'s command set.

use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// One widget in a `Spec`. `TextEdit` and `Button` carry an
/// extension-assigned `id` so `Clicked`/`Changed` can name the widget that
/// produced them; `Label` has no interaction, so no id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Widget<'a> {
    Label { text: &'a str },
    TextEdit { id: u32, text: &'a str },
    Button { id: u32, label: &'a str },
}

const TAG_LABEL: u8 = 0;
const TAG_TEXT_EDIT: u8 = 1;
const TAG_BUTTON: u8 = 2;

impl<'a> Widget<'a> {
    fn encode_into(&self, writer: Writer) -> Writer {
        match self {
            Widget::Label { text } => writer.u8(TAG_LABEL).str(text),
            Widget::TextEdit { id, text } => writer.u8(TAG_TEXT_EDIT).u32(*id).str(text),
            Widget::Button { id, label } => writer.u8(TAG_BUTTON).u32(*id).str(label),
        }
    }

    fn decode(reader: &mut Reader<'a>) -> Result<Self, DecodeError> {
        let tag = reader.read_u8()?;
        match tag {
            TAG_LABEL => Ok(Widget::Label {
                text: reader.read_str()?,
            }),
            TAG_TEXT_EDIT => Ok(Widget::TextEdit {
                id: reader.read_u32()?,
                text: reader.read_str()?,
            }),
            TAG_BUTTON => Ok(Widget::Button {
                id: reader.read_u32()?,
                label: reader.read_str()?,
            }),
            _ => Err(DecodeError::InvalidTag {
                message: "ui widget",
                tag,
            }),
        }
    }
}

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

/// Published back to the owning extension when a `Button` widget is clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clicked {
    pub id: u32,
}

impl Message for Clicked {
    const TOPIC: &'static str = "ui/clicked";
}

impl EncodeMessage for Clicked {
    fn encode(&self) -> Vec<u8> {
        Writer::new().u32(self.id).finish()
    }
}

impl<'a> DecodeMessage<'a> for Clicked {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let message = Self {
            id: reader.read_u32()?,
        };
        reader.finish()?;
        Ok(message)
    }
}

/// Published back to the owning extension when a `TextEdit` widget's text
/// changes; carries the field's full new content, not a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed<'a> {
    pub id: u32,
    pub text: &'a str,
}

impl Message for Changed<'_> {
    const TOPIC: &'static str = "ui/changed";
}

impl EncodeMessage for Changed<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.id)
            .bytes(self.text.as_bytes())
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Changed<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let id = reader.read_u32()?;
        let text = reader.read_str_rest()?;
        Ok(Self { id, text })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_with_every_widget_kind_round_trips() {
        let spec = Spec {
            title: "notes",
            widgets: vec![
                Widget::TextEdit {
                    id: 1,
                    text: "buy milk",
                },
                Widget::Button {
                    id: 2,
                    label: "Add",
                },
                Widget::Label {
                    text: "existing note",
                },
            ],
        };
        assert_eq!(Spec::decode(&spec.encode()), Ok(spec));
    }

    #[test]
    fn empty_spec_round_trips() {
        let spec = Spec {
            title: "",
            widgets: vec![],
        };
        assert_eq!(Spec::decode(&spec.encode()), Ok(spec));
    }

    #[test]
    fn unknown_widget_tag_is_rejected() {
        let payload = Writer::new().str("t").u16(1).u8(255).finish();
        assert_eq!(
            Spec::decode(&payload),
            Err(DecodeError::InvalidTag {
                message: "ui widget",
                tag: 255
            })
        );
    }

    #[test]
    fn clicked_round_trips() {
        let clicked = Clicked { id: 42 };
        assert_eq!(Clicked::decode(&clicked.encode()), Ok(clicked));
    }

    #[test]
    fn changed_round_trips() {
        let changed = Changed {
            id: 7,
            text: "new text",
        };
        assert_eq!(Changed::decode(&changed.encode()), Ok(changed));
    }
}
