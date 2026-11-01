//! Lowercase hex, the payload notation of the conformance vectors and the
//! devtools channel.

/// Two lowercase digits per byte.
pub fn format(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Bytes from hex digits, either case; `None` for an odd length or a
/// non-hex digit. `-` is the empty payload, as in the vectors.
pub fn parse(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if text == "-" {
        return Some(Vec::new());
    }
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}
