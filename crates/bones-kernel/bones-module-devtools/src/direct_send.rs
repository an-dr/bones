//! `/send`: a direct send made for an HTTP client.
//!
//! A send runs the target's handler synchronously, so it must run where
//! every other send runs: on the frame thread, inside a delivery. The route
//! therefore publishes the request to the module itself on its own topic,
//! and the module makes the call when the bus delivers it.

use bones_kernel_infra::bus::{Envelope, Registry};
use bones_messages::{DecodeError, Reader, Writer};

use crate::devtools::ENDPOINT;

/// The module's own topic for a send waiting to be made.
pub(crate) const TOPIC: &str = "devtools/send";

/// `/send` replies by request id: the reply, or why there is none.
pub(crate) type Replies =
    std::sync::Arc<std::sync::Mutex<std::collections::HashMap<u32, Result<Vec<u8>, String>>>>;

/// One send to make: who asked, where to, and what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectSend {
    pub(crate) id: u32,
    pub(crate) endpoint: String,
    pub(crate) payload: Vec<u8>,
}

impl DirectSend {
    /// The envelope the route publishes.
    pub(crate) fn envelope(&self) -> Envelope {
        Envelope {
            topic: TOPIC.to_string(),
            sender: ENDPOINT.to_string(),
            correlation: None,
            payload: Writer::new()
                .u32(self.id)
                .str(&self.endpoint)
                .bytes(&self.payload)
                .finish(),
        }
    }

    /// The send an envelope on [`TOPIC`] carries.
    pub(crate) fn decode(payload: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            id: reader.read_u32()?,
            endpoint: reader.read_str()?.to_string(),
            payload: reader.read_rest().to_vec(),
        })
    }

    /// Makes the send, as `devtools`; the reply, or why there is none.
    pub(crate) fn call(&self, registry: &Registry) -> Result<Vec<u8>, String> {
        registry
            .call(ENDPOINT, &self.endpoint, &self.payload)
            .map_err(|error| format!("{error:?}"))
    }
}

#[cfg(test)]
mod tests;
