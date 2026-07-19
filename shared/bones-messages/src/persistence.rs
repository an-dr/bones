//! `persistence/*`: lets a WASM extension save and restore its own state
//! through the trusted `persistence` native module, since extensions have
//! no OS/file API of their own (the module-vs-extension trust split).
//!
//! Save is pub/sub, fire-and-forget (`Save`, below) — the sender is taken
//! from the envelope itself (host-stamped, not guest-suppliable), so one
//! extension can never overwrite, or — the same guarantee, since a direct
//! `send`'s `sender` is exactly as host-stamped — ever load, another's
//! save file. Load has no typed
//! message: it's a direct `send` (ADR-010) to the well-known [`ENDPOINT`]
//! name with an empty payload; the reply is the raw saved bytes, or empty
//! if there was nothing to load — see `core/persistence`'s own docs for
//! why an empty reply is ambiguous between "never saved" and "saved
//! nothing", and why that's an accepted simplification for now.

use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// The bus endpoint name `persistence/*`'s native module registers under
/// — the `send` target for a direct load call.
pub const ENDPOINT: &str = "persistence";

/// Saves `bytes` as the calling extension's own state, replacing whatever
/// was saved before. Fire-and-forget: publish and move on, no reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Save<'a> {
    pub bytes: &'a [u8],
}

impl Message for Save<'_> {
    const TOPIC: &'static str = "persistence/save";
}

impl EncodeMessage for Save<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().bytes(self.bytes).finish()
    }
}

impl<'a> DecodeMessage<'a> for Save<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            bytes: reader.read_rest(),
        })
    }
}

#[cfg(test)]
mod tests;
