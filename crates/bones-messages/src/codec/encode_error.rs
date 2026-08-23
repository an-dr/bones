#[derive(Debug, PartialEq, Eq)]
/// A value that cannot be represented in the wire format.
///
/// Only the length-prefixed fields can fail, and only by overflowing their
/// prefix. Every fixed-width field encodes any value of its type, which is why
/// this enum is smaller than [`super::DecodeError`]: writing is total except
/// where a length has to fit.
pub enum EncodeError {
    /// A string field's UTF-8 length exceeded the `u16` prefix that frames it.
    StringTooLong,
    /// A blob field's length exceeded the `u32` prefix that frames it.
    BlobTooLong,
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::StringTooLong => {
                write!(
                    f,
                    "string exceeds the u16::MAX bytes its length prefix can frame"
                )
            }
            EncodeError::BlobTooLong => {
                write!(
                    f,
                    "blob exceeds the u32::MAX bytes its length prefix can frame"
                )
            }
        }
    }
}

impl std::error::Error for EncodeError {}
