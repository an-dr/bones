//! Fixed-layout little-endian byte encoding for bus payloads (ADR-001: any
//! guest language must be able to produce/consume these, so no serde).

#[derive(Default)]
pub struct Writer(Vec<u8>);

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn u8(mut self, v: u8) -> Self {
        self.0.push(v);
        self
    }

    pub fn u32(mut self, v: u32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn i32(mut self, v: i32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn bytes(mut self, v: &[u8]) -> Self {
        self.0.extend_from_slice(v);
        self
    }

    pub fn finish(self) -> Vec<u8> {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Truncated,
    TrailingBytes,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Truncated => write!(f, "payload ended before an expected field"),
            Error::TrailingBytes => write!(f, "payload has bytes left after parsing"),
        }
    }
}

/// Lets callers that model their own errors as `String` use `?` directly on
/// `Reader` methods instead of `.map_err(|e| e.to_string())` at every call.
impl From<Error> for String {
    fn from(err: Error) -> Self {
        err.to_string()
    }
}

/// Bounds-checked reader over a byte slice; every method advances past what
/// it read.
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub fn read_u8(&mut self) -> Result<u8, Error> {
        self.read_array::<1>().map(|b| b[0])
    }

    pub fn read_u32(&mut self) -> Result<u32, Error> {
        self.read_array::<4>().map(u32::from_le_bytes)
    }

    pub fn read_i32(&mut self) -> Result<i32, Error> {
        self.read_array::<4>().map(i32::from_le_bytes)
    }

    /// Every remaining byte, regardless of length.
    pub fn read_rest(&mut self) -> &'a [u8] {
        let rest = &self.bytes[self.pos..];
        self.pos = self.bytes.len();
        rest
    }

    /// Errors if any bytes remain unread — call after a fixed-shape payload
    /// should be fully consumed.
    pub fn finish(self) -> Result<(), Error> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::TrailingBytes)
        }
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        if self.bytes.len() - self.pos < N {
            return Err(Error::Truncated);
        }
        let array: [u8; N] = self.bytes[self.pos..self.pos + N].try_into().unwrap();
        self.pos += N;
        Ok(array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_produces_little_endian_bytes_in_call_order() {
        let bytes = Writer::new().u8(1).u32(2).i32(-1).bytes(b"xy").finish();
        assert_eq!(bytes, [1, 2, 0, 0, 0, 255, 255, 255, 255, b'x', b'y']);
    }

    #[test]
    fn reader_reads_back_what_writer_wrote() {
        let bytes = Writer::new().u32(7).i32(-100).finish();
        let mut r = Reader::new(&bytes);
        assert_eq!(r.read_u32(), Ok(7));
        assert_eq!(r.read_i32(), Ok(-100));
        assert_eq!(r.finish(), Ok(()));
    }

    #[test]
    fn reader_rejects_a_truncated_field() {
        let mut r = Reader::new(&[0, 0]);
        assert_eq!(r.read_u32(), Err(Error::Truncated));
    }

    #[test]
    fn finish_rejects_trailing_bytes() {
        let mut r = Reader::new(&[1, 2, 3, 4, 5]);
        r.read_u32().unwrap();
        assert_eq!(r.finish(), Err(Error::TrailingBytes));
    }

    #[test]
    fn read_rest_takes_every_remaining_byte() {
        let mut r = Reader::new(&[1, 2, 3, 4, 5]);
        r.read_u8().unwrap();
        assert_eq!(r.read_rest(), &[2, 3, 4, 5]);
    }

    #[test]
    fn error_converts_to_a_string_via_question_mark() {
        fn parse(payload: &[u8]) -> Result<u32, String> {
            Ok(Reader::new(payload).read_u32()?)
        }
        assert_eq!(parse(&[1, 0, 0, 0]), Ok(1));
        assert!(parse(&[]).is_err());
    }
}
