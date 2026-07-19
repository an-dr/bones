//! Fixed-layout little-endian byte encoding for bus payloads (ADR-001: any
//! guest language must be able to produce/consume these, so no serde).

mod decode_error;
mod reader;
mod writer;

pub use decode_error::DecodeError;
pub use reader::Reader;
pub use writer::Writer;

#[cfg(test)]
mod tests;
