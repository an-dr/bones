//! Standard base64 with padding, for the image an MCP tool returns.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            match i <= chunk.len() {
                true => text.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char),
                false => text.push('='),
            }
        }
    }
    text
}

#[cfg(test)]
mod tests;
