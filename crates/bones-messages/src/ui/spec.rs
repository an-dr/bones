use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use super::{Node, Root};

/// One owner's whole interface for one frame (ADR-048): published on
/// `ui/spec` by an extension, or handed to the `ui` service by a native
/// module. Not sending one this frame means nothing of that owner's is
/// drawn this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// Where the tree is drawn.
    pub root: Root,
    /// The tree; usually a container.
    pub body: Node,
}

impl Message for Spec {
    const TOPIC: &'static str = "ui/spec";
}

impl EncodeMessage for Spec {
    fn encode(&self) -> Vec<u8> {
        self.body
            .encode_into(self.root.encode_into(Writer::new()))
            .finish()
    }
}

impl<'a> DecodeMessage<'a> for Spec {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let root = Root::decode(&mut reader)?;
        let body = Node::decode(&mut reader, 1)?;
        reader.finish()?;
        Ok(Self { root, body })
    }
}
