use super::DecodeError;

/// Bounds-checked reader over a byte slice; every method advances past what
/// it read.
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Starts reading at the beginning of `bytes`.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Reads one unsigned byte.
    pub fn read_u8(&mut self) -> Result<u8, DecodeError> {
        self.read_array::<1>().map(|b| b[0])
    }

    /// Reads a little-endian `u32`.
    pub fn read_u32(&mut self) -> Result<u32, DecodeError> {
        self.read_array::<4>().map(u32::from_le_bytes)
    }

    /// Reads a little-endian `i32`.
    pub fn read_i32(&mut self) -> Result<i32, DecodeError> {
        self.read_array::<4>().map(i32::from_le_bytes)
    }

    /// Reads a little-endian `f32`.
    pub fn read_f32(&mut self) -> Result<f32, DecodeError> {
        self.read_array::<4>().map(f32::from_le_bytes)
    }

    /// Reads a little-endian `u16`.
    pub fn read_u16(&mut self) -> Result<u16, DecodeError> {
        self.read_array::<2>().map(u16::from_le_bytes)
    }

    /// Reads a `u16`-length-prefixed UTF-8 string written by `Writer::str`.
    pub fn read_str(&mut self) -> Result<&'a str, DecodeError> {
        let len = self.read_u16()? as usize;
        if self.bytes.len() - self.pos < len {
            return Err(DecodeError::Truncated);
        }
        let slice = &self.bytes[self.pos..self.pos + len];
        self.pos += len;
        std::str::from_utf8(slice).map_err(|_| DecodeError::InvalidUtf8)
    }

    /// Every remaining byte, regardless of length.
    pub fn read_rest(&mut self) -> &'a [u8] {
        let rest = &self.bytes[self.pos..];
        self.pos = self.bytes.len();
        rest
    }

    /// Reads every remaining byte as UTF-8 without allocating.
    pub fn read_str_rest(&mut self) -> Result<&'a str, DecodeError> {
        std::str::from_utf8(self.read_rest()).map_err(|_| DecodeError::InvalidUtf8)
    }

    /// Errors if any bytes remain unread — call after a fixed-shape payload
    /// should be fully consumed.
    pub fn finish(self) -> Result<(), DecodeError> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes)
        }
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        if self.bytes.len() - self.pos < N {
            return Err(DecodeError::Truncated);
        }
        let array: [u8; N] = self.bytes[self.pos..self.pos + N].try_into().unwrap();
        self.pos += N;
        Ok(array)
    }
}
