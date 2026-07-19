use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

/// Loads a Tiled `.tmx` map (XML bytes, as exported by the Tiled editor)
/// without copying them — parsing happens in `game-core`, not on the wire
/// (ADR-019: `tiled` is parsing only, rendering stays `gfx/*`). An object
/// layer named `"Collision"` becomes static colliders; every other layer
/// is ignored (this module carries no rendering authority).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadTilemap<'a> {
    /// Raw `.tmx` file bytes.
    pub tmx_bytes: &'a [u8],
}

impl Message for LoadTilemap<'_> {
    const TOPIC: &'static str = "game-core/load-tilemap";
}

impl EncodeMessage for LoadTilemap<'_> {
    fn encode(&self) -> Vec<u8> {
        Writer::new().bytes(self.tmx_bytes).finish()
    }
}

impl<'a> DecodeMessage<'a> for LoadTilemap<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        Ok(Self {
            tmx_bytes: reader.read_rest(),
        })
    }
}
