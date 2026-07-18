//! Typed core-defined messages shared by native components and WASM guests.
//! The raw bus remains open and byte-oriented; this crate defines the topic
//! and wire contract only for messages owned by bones (ADR-016).

pub mod audio;
pub mod codec;
pub mod gfx;
pub mod input;
pub mod lifecycle;
pub mod persistence;
pub mod tick;
pub mod ui;

pub use codec::{DecodeError, Reader, Writer};

/// Identifies the exact bus topic belonging to a typed message.
pub trait Message {
    const TOPIC: &'static str;
}

/// Encodes a typed message into its stable bus payload representation.
pub trait EncodeMessage: Message {
    fn encode(&self) -> Vec<u8>;
}

/// Decodes a typed message from its bus payload without requiring ownership.
pub trait DecodeMessage<'a>: Message + Sized {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError>;
}
