#[derive(Debug, PartialEq, Eq)]
/// A malformed core-defined message payload.
pub enum DecodeError {
    /// The payload ended before a complete field could be read.
    Truncated,
    /// A fixed-shape message contained bytes after its final field.
    TrailingBytes,
    /// A tagged enum contained a value not defined by its message contract.
    InvalidTag { message: &'static str, tag: u8 },
    /// A string field was not valid UTF-8.
    InvalidUtf8,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Truncated => write!(f, "payload ended before an expected field"),
            DecodeError::TrailingBytes => write!(f, "payload has bytes left after decoding"),
            DecodeError::InvalidTag { message, tag } => write!(f, "unknown {message} tag {tag}"),
            DecodeError::InvalidUtf8 => write!(f, "payload contains invalid UTF-8"),
        }
    }
}

impl std::error::Error for DecodeError {}
