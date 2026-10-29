use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bones_kernel_infra::bus::Envelope;
use bones_messages::gfx::Command;

/// Draw commands by sender, one line each.
pub(crate) type Batches = Arc<Mutex<BTreeMap<String, Vec<String>>>>;

/// Keeps what each sender drew in its latest frame, for `/frame`.
///
/// - A frame ends at `core/tick`. Each sender that published `gfx/*` since
///   the previous tick replaces its batch; a sender that published nothing
///   keeps its old one, as the renderer keeps compositing it.
/// - A command that does not decode is kept as the decode error, so a
///   malformed draw is visible rather than silently missing.
pub(crate) struct Frames {
    drawing: BTreeMap<String, Vec<String>>,
    last: Batches,
}

impl Frames {
    pub(crate) fn new() -> Self {
        Self {
            drawing: BTreeMap::new(),
            last: Batches::default(),
        }
    }

    /// The batches `/frame` reads.
    pub(crate) fn last(&self) -> Batches {
        Arc::clone(&self.last)
    }

    /// Records a `gfx/*` envelope into its sender's batch in progress.
    pub(crate) fn record(&mut self, envelope: &Envelope) {
        let line = match Command::decode(&envelope.topic, &envelope.payload) {
            Ok(Some(command)) => format!("{command:?}"),
            Ok(None) => return,
            Err(error) => format!("{} (does not decode: {error})", envelope.topic),
        };
        self.drawing
            .entry(envelope.sender.clone())
            .or_default()
            .push(line);
    }

    /// Ends the frame: every batch in progress becomes its sender's latest.
    pub(crate) fn end_frame(&mut self) {
        if self.drawing.is_empty() {
            return;
        }
        let mut last = self.last.lock().unwrap();
        for (sender, batch) in std::mem::take(&mut self.drawing) {
            last.insert(sender, batch);
        }
    }
}

/// The text `/frame` answers: a `[sender]` header, then one command per line.
pub(crate) fn render(batches: &BTreeMap<String, Vec<String>>) -> String {
    let mut text = String::new();
    for (sender, batch) in batches {
        text.push_str(&format!("[{sender}]\n"));
        for line in batch {
            text.push_str(line);
            text.push('\n');
        }
    }
    text
}

#[cfg(test)]
mod tests;
