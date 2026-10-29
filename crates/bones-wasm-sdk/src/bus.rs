//! Typed calls over the raw `host-api` imports.
//!
//! - The topic of every call comes from the message type, so a misspelt topic
//!   fails to compile instead of reaching no subscriber.
//! - Each core endpoint gets one call shaped like its contract
//!   (`wit/wire-format.md`, "Direct-send endpoints"), so a caller never builds
//!   an endpoint payload by hand.
//! - The raw imports stay available in [`crate::bindings`] for an
//!   extension's own topics, which bones does not type.

use bones_messages::persistence::Save;
use bones_messages::web::Command;
use bones_messages::{EncodeMessage, Message};

use crate::bindings::bones::extension::host_api;

pub use crate::bindings::bones::extension::host_api::SendError;

/// Publishes a core message on its own topic.
pub fn publish<M: EncodeMessage>(message: &M) {
    host_api::publish(M::TOPIC, &message.encode());
}

/// Subscribes to the topic of message type `M`; deliveries arrive in
/// `on_message` and decode with [`crate::Inbound::decode`].
pub fn subscribe<M: Message>() {
    host_api::subscribe(M::TOPIC);
}

/// This extension's saved state, or empty when nothing was saved.
pub fn load_state() -> Result<Vec<u8>, SendError> {
    host_api::send(bones_messages::persistence::ENDPOINT, &[])
}

/// Replaces this extension's saved state. Fire-and-forget: there is no reply.
pub fn save_state(bytes: &[u8]) {
    publish(&Save { bytes });
}

/// A file's bytes from the directory the application granted, or empty when
/// there is nothing to return. Fails with `UnknownEndpoint` when no
/// directory was granted.
pub fn read_file(path: &str) -> Result<Vec<u8>, SendError> {
    host_api::send(bones_messages::files::ENDPOINT, path.as_bytes())
}

/// Sends a command to the `web` endpoint: open, close, or navigate a panel,
/// or post JSON to its page. Fails with `UnknownEndpoint` when the engine
/// runs without web panels.
pub fn web(command: &Command) -> Result<(), SendError> {
    host_api::send(bones_messages::web::ENDPOINT, &command.encode()).map(|_| ())
}
