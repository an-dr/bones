/// One envelope as the recording keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Recorded {
    /// Milliseconds since the channel was created.
    pub(crate) ms: u128,
    pub(crate) topic: String,
    pub(crate) sender: String,
    /// Readable summary of the payload (`recording::preview`).
    pub(crate) preview: String,
    /// The whole payload, or `None` when it was redacted.
    pub(crate) payload: Option<Vec<u8>>,
}

impl Recorded {
    /// The text form: elapsed time, topic, sender, and preview, tab-separated.
    pub(crate) fn line(&self) -> String {
        format!(
            "{:>7}ms\t{}\t{}\t{}",
            self.ms, self.topic, self.sender, self.preview
        )
    }

    /// The JSON form, with the whole payload as hex.
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "ms": self.ms as u64,
            "topic": self.topic,
            "sender": self.sender,
            "preview": self.preview,
            "hex": self.payload.as_deref().map(hex),
        })
    }
}

/// Lowercase hex, the payload notation of the conformance vectors.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
