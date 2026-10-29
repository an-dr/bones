use std::collections::BTreeMap;

use bones_kernel::bus::Envelope;
use bones_messages::tick::Tick;
use bones_messages::Message;

/// What the recorder has heard since [`crate::App`] last collected it.
///
/// A frame ends at `core/tick`, as in the renderer: a sender's draws since
/// the previous tick become its batch, and a sender that drew nothing keeps
/// its old one.
#[derive(Default)]
pub(crate) struct Recording {
    /// Every envelope except ticks, oldest first.
    pub(crate) envelopes: Vec<Envelope>,
    /// Each sender's `gfx/*` envelopes since the last tick.
    drawing: BTreeMap<String, Vec<Envelope>>,
    /// Each sender's most recent completed `gfx/*` batch.
    frame: BTreeMap<String, Vec<Envelope>>,
}

impl Recording {
    pub(crate) fn hear(&mut self, envelope: &Envelope) {
        if envelope.topic == Tick::TOPIC {
            let drawn = std::mem::take(&mut self.drawing);
            self.frame.extend(drawn);
            return;
        }
        if envelope.topic.starts_with("gfx/") {
            self.drawing
                .entry(envelope.sender.clone())
                .or_default()
                .push(envelope.clone());
        }
        self.envelopes.push(envelope.clone());
    }

    /// Each sender's latest batch, counting one still in progress: what the
    /// renderer would composite if it presented now.
    pub(crate) fn latest_frame(&self) -> BTreeMap<String, Vec<Envelope>> {
        let mut frame = self.frame.clone();
        frame.extend(self.drawing.clone());
        frame
    }
}
