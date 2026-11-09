use crate::{EncodeMessage, Message};

use super::{Changed, Clicked};

/// Something a user did to one widget, as the ui module reports it to the
/// widget's owner (ADR-048): returned to a native owner, published as the
/// matching `ui/*` message to an extension owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A button-like widget was activated.
    Clicked {
        /// The widget's id.
        id: u32,
    },
    /// A text field's contents changed.
    Changed {
        /// The widget's id.
        id: u32,
        /// The new contents.
        text: String,
    },
}

impl Event {
    /// The id of the widget the event belongs to.
    pub fn id(&self) -> u32 {
        match self {
            Event::Clicked { id } | Event::Changed { id, .. } => *id,
        }
    }

    /// The event as the topic and payload published to an extension owner.
    pub fn to_message(&self) -> (&'static str, Vec<u8>) {
        match self {
            Event::Clicked { id } => (Clicked::TOPIC, Clicked { id: *id }.encode()),
            Event::Changed { id, text } => (
                Changed::TOPIC,
                Changed {
                    id: *id,
                    text: text.as_str(),
                }
                .encode(),
            ),
        }
    }
}
