//! How a recorded envelope is rendered for a human reading the recording.

use bones_kernel_infra::bus::Envelope;
use bones_messages::web::PageMessage;
use bones_messages::{DecodeMessage, Message};

/// Decides which envelopes keep their payload out of the recording.
pub(crate) type Redact = dyn Fn(&Envelope) -> bool + Send + Sync;

/// Longest payload preview kept per envelope.
const PREVIEW: usize = 300;

/// Page messages are JSON and stay readable as they are; everything else is
/// the binary codec, so it gets its length and whatever text it contains.
pub(crate) fn preview(envelope: &Envelope, redact: Option<&Redact>) -> String {
    if redact.is_some_and(|redact| redact(envelope)) {
        return format!("[{} bytes redacted]", envelope.payload.len());
    }
    if envelope.topic == PageMessage::TOPIC {
        if let Ok(message) = PageMessage::decode(&envelope.payload) {
            return truncate(&format!(
                "{}/{} {}",
                message.owner, message.panel, message.json
            ));
        }
    }
    let text: String = envelope
        .payload
        .iter()
        .map(|byte| match byte.is_ascii_graphic() || *byte == b' ' {
            true => *byte as char,
            false => '.',
        })
        .collect();
    truncate(&format!("[{} bytes] {text}", envelope.payload.len()))
}

fn truncate(text: &str) -> String {
    match text.char_indices().nth(PREVIEW) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}
