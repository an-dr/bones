/// Builder for the fixed-layout little-endian core-message encoding.
#[derive(Default)]
pub struct Writer(Vec<u8>);

impl Writer {
    /// Starts an empty payload.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one unsigned byte.
    pub fn u8(mut self, v: u8) -> Self {
        self.0.push(v);
        self
    }

    /// Appends a little-endian `u32`.
    pub fn u32(mut self, v: u32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Appends a little-endian `i32`.
    pub fn i32(mut self, v: i32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Appends a little-endian `f32`.
    pub fn f32(mut self, v: f32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Appends a little-endian `u16`.
    pub fn u16(mut self, v: u16) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// Appends bytes without a length prefix.
    pub fn bytes(mut self, v: &[u8]) -> Self {
        self.0.extend_from_slice(v);
        self
    }

    /// Appends a `u16`-length-prefixed UTF-8 string — for messages with more
    /// than one variable-length field, where `read_rest` can't tell them
    /// apart. Panics if `v` is longer than `u16::MAX` bytes.
    pub fn str(self, v: &str) -> Self {
        let len: u16 = v.len().try_into().expect("string exceeds u16::MAX bytes");
        self.u16(len).bytes(v.as_bytes())
    }

    /// Returns the completed payload.
    pub fn finish(self) -> Vec<u8> {
        self.0
    }
}
