//! A message vocabulary bones does not own.
//!
//! The bus is byte-oriented and open (ADR-016): bones defines topics and
//! payloads for the things *it* owns, and anything else on the bus is opaque to
//! it. That is the extension point this crate demonstrates — an embedder adding
//! a native capability also gets to define how it is spoken to, without asking
//! bones for a topic or waiting for a release.
//!
//! Two things make it work, and both are deliberate properties of the engine
//! rather than luck:
//!
//! - `bones-messages` has **no dependencies**, so it compiles for the native
//!   host and for `wasm32-wasip2`. This crate depends on it for the codec and
//!   inherits that, which is why the same types can be linked into a native
//!   module and into a WASM guest.
//! - The codec primitives (`Reader`, `Writer`) are public, so a custom
//!   vocabulary is encoded exactly the way core messages are — same
//!   little-endian layout, same framing rules, same failure modes
//!   (`wit/wire-format.md`).
//!
//! A guest in another language would reimplement these two types from that
//! document. Nothing here is Rust-only except the convenience.

use bones_messages::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// The bus endpoint the native module registers under, and the target of the
/// direct `send` that carries [`FactsRequest`].
///
/// Not a topic: a request that expects an answer is a direct send (ADR-010),
/// which completes inside the call. Publishing would be the wrong shape — the
/// caller wants a reply, not a broadcast.
pub const ENDPOINT: &str = "host-facts";

/// Which fact the caller wants.
///
/// An enum rather than one message per fact, so adding a fact later costs a tag
/// rather than a topic — the same reason `game-core/entity-op` is one tagged
/// union instead of eleven topics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fact<'a> {
    /// The machine's hostname.
    Hostname,
    /// The directory the engine was launched from.
    WorkingDirectory,
    /// One named environment variable.
    EnvironmentVariable(&'a str),
}

const TAG_HOSTNAME: u8 = 0;
const TAG_WORKING_DIRECTORY: u8 = 1;
const TAG_ENVIRONMENT_VARIABLE: u8 = 2;

/// A request for one fact, sent to [`ENDPOINT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactsRequest<'a> {
    /// What is being asked for.
    pub fact: Fact<'a>,
}

impl Message for FactsRequest<'_> {
    // A direct send carries no topic (see `wit/extension.wit`'s `on-message`), but
    // `Message` is what gives `encode`/`decode` their pairing, so the constant
    // names the endpoint instead. A custom vocabulary is free to do this; the
    // core messages, which are all published, are not.
    const TOPIC: &'static str = ENDPOINT;
}

impl EncodeMessage for FactsRequest<'_> {
    fn encode(&self) -> Vec<u8> {
        match self.fact {
            Fact::Hostname => Writer::new().u8(TAG_HOSTNAME).finish(),
            Fact::WorkingDirectory => Writer::new().u8(TAG_WORKING_DIRECTORY).finish(),
            // `bytes`, not `str`: nothing follows, so the length prefix would
            // be dead weight (`wit/wire-format.md`'s framing rules).
            Fact::EnvironmentVariable(name) => Writer::new()
                .u8(TAG_ENVIRONMENT_VARIABLE)
                .bytes(name.as_bytes())
                .finish(),
        }
    }
}

impl<'a> DecodeMessage<'a> for FactsRequest<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let fact = match reader.read_u8()? {
            TAG_HOSTNAME => Fact::Hostname,
            TAG_WORKING_DIRECTORY => Fact::WorkingDirectory,
            TAG_ENVIRONMENT_VARIABLE => Fact::EnvironmentVariable(reader.read_str_rest()?),
            tag => {
                return Err(DecodeError::InvalidTag {
                    message: "host fact",
                    tag,
                })
            }
        };
        Ok(Self { fact })
    }
}

/// The reply: the fact's value, or empty if the host could not determine it.
///
/// Empty rather than an error variant, matching what `persistence` and `files`
/// already do — the caller's next move is the same whether the value was
/// missing or unreadable, so distinguishing them would be a distinction nobody
/// acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactsReply<'a> {
    /// The value, or `""`.
    pub value: &'a str,
}

impl Message for FactsReply<'_> {
    const TOPIC: &'static str = ENDPOINT;
}

impl EncodeMessage for FactsReply<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().bytes(self.value.as_bytes()).finish()
    }
}

impl<'a> DecodeMessage<'a> for FactsReply<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            value: reader.read_str_rest()?,
        })
    }
}

#[cfg(test)]
mod tests;
