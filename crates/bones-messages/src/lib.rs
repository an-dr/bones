//! Typed core-defined messages shared by native components and WASM guests.
//! The raw bus remains open and byte-oriented; this crate defines the topic
//! and wire contract only for messages owned by bones (ADR-016).

#![warn(missing_docs)]

pub mod audio;
pub mod catalogue;
pub mod codec;
pub mod desktop;
pub mod extension_control;
pub mod files;
pub mod game_core;
pub mod gfx;
pub mod input;
pub mod instance;
pub mod lifecycle;
pub mod os;
pub mod persistence;
pub mod renderer;
pub mod tick;
pub mod ui;
pub mod upgrader;
pub mod web;
pub mod window;

pub use codec::{DecodeError, EncodeError, Reader, Writer};

/// Identifies the exact bus topic belonging to a typed message.
pub trait Message {
    /// The exact bus topic this message travels on.
    const TOPIC: &'static str;
}

/// Encodes a typed message into its stable bus payload representation.
pub trait EncodeMessage: Message {
    /// The payload bytes, as `wit/wire-format.md` specifies them.
    fn encode(&self) -> Vec<u8>;
}

/// Decodes a typed message from its bus payload without requiring ownership.
pub trait DecodeMessage<'a>: Message + Sized {
    /// Reads a payload, failing on anything the specification does not allow, including trailing bytes.
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError>;
}
