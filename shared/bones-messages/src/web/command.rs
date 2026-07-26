use crate::{DecodeError, Reader, Writer};

use super::{ClosePanel, Navigate, OpenPanel, PanelSource, SendJson};

/// Any direct command accepted by the `web` endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command<'a> {
    Open(OpenPanel<'a>),
    Close(ClosePanel<'a>),
    Navigate(Navigate<'a>),
    SendJson(SendJson<'a>),
}

impl Command<'_> {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Open(open) => Writer::new()
                .u8(0)
                .str(open.panel)
                .u8(open.source.tag())
                .bytes(open.source.value().as_bytes())
                .finish(),
            Self::Close(close) => Writer::new().u8(1).bytes(close.panel.as_bytes()).finish(),
            Self::Navigate(navigate) => Writer::new()
                .u8(2)
                .str(navigate.panel)
                .bytes(navigate.url.as_bytes())
                .finish(),
            Self::SendJson(message) => Writer::new()
                .u8(3)
                .str(message.panel)
                .bytes(message.json.as_bytes())
                .finish(),
        }
    }
}

impl<'a> Command<'a> {
    pub fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        match reader.read_u8()? {
            0 => {
                let panel = reader.read_str()?;
                let source_tag = reader.read_u8()?;
                let value = reader.read_str_rest()?;
                Ok(Self::Open(OpenPanel {
                    panel,
                    source: PanelSource::from_tag(source_tag, value)?,
                }))
            }
            1 => Ok(Self::Close(ClosePanel {
                panel: reader.read_str_rest()?,
            })),
            2 => Ok(Self::Navigate(Navigate {
                panel: reader.read_str()?,
                url: reader.read_str_rest()?,
            })),
            3 => Ok(Self::SendJson(SendJson {
                panel: reader.read_str()?,
                json: reader.read_str_rest()?,
            })),
            tag => Err(DecodeError::InvalidTag {
                message: "web command",
                tag,
            }),
        }
    }
}
