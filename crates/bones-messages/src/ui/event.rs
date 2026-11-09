use crate::{EncodeMessage, Message};

use super::{Changed, Clicked, Selected, Toggled, ValueChanged};

/// Something a user did to one widget, as the ui module reports it to the
/// widget's owner (ADR-048): returned to a native owner, published as the
/// matching `ui/*` message to an extension owner.
#[derive(Debug, Clone, PartialEq)]
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
    /// An option of a choice widget was chosen.
    Selected {
        /// The widget's id.
        id: u32,
        /// The option's index.
        index: u32,
    },
    /// An on/off widget changed state.
    Toggled {
        /// The widget's id.
        id: u32,
        /// The new state.
        on: bool,
    },
    /// A value widget was moved.
    ValueChanged {
        /// The widget's id.
        id: u32,
        /// The new value.
        value: f32,
    },
}

impl Event {
    /// The id of the widget the event belongs to.
    pub fn id(&self) -> u32 {
        match self {
            Event::Clicked { id }
            | Event::Changed { id, .. }
            | Event::Selected { id, .. }
            | Event::Toggled { id, .. }
            | Event::ValueChanged { id, .. } => *id,
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
            Event::Selected { id, index } => (
                Selected::TOPIC,
                Selected {
                    id: *id,
                    index: *index,
                }
                .encode(),
            ),
            Event::Toggled { id, on } => (Toggled::TOPIC, Toggled { id: *id, on: *on }.encode()),
            Event::ValueChanged { id, value } => (
                ValueChanged::TOPIC,
                ValueChanged {
                    id: *id,
                    value: *value,
                }
                .encode(),
            ),
        }
    }
}
