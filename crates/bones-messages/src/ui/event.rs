use crate::{EncodeMessage, Message};

use super::{Changed, Clicked, Dropped, KeyPressed, Selected, Submitted, Toggled, ValueChanged};

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
    /// A `draggable` container was dropped on a `drop-target` one.
    Dropped {
        /// The drop target's id.
        id: u32,
        /// The dropped container's id.
        payload: u32,
    },
    /// Enter was pressed in a text field.
    Submitted {
        /// The field's id.
        id: u32,
    },
    /// A key no widget took, for the `Screen` owner; its id is always 0.
    KeyPressed {
        /// The key's name, as egui names it: `Escape`, `ArrowLeft`.
        key: String,
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
            | Event::ValueChanged { id, .. }
            | Event::Dropped { id, .. }
            | Event::Submitted { id } => *id,
            Event::KeyPressed { .. } => 0,
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
            Event::Dropped { id, payload } => (
                Dropped::TOPIC,
                Dropped {
                    id: *id,
                    payload: *payload,
                }
                .encode(),
            ),
            Event::Submitted { id } => (Submitted::TOPIC, Submitted { id: *id }.encode()),
            Event::KeyPressed { key } => (KeyPressed::TOPIC, KeyPressed { key }.encode()),
        }
    }
}
