use crate::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use super::TilesetImage;

/// Loads a Tiled `.tmx` map (XML bytes, as exported by the Tiled editor,
/// embedded tilesets only — see `TilesetImage`) — parsing happens in
/// `game-core`, not on the wire (ADR-019: `tiled` is parsing only). An
/// object layer named `"Collision"` becomes static colliders; a tile
/// layer named `"Ground"` is drawn every tick at the background layer,
/// using whichever `tileset_images` match its tilesets by name (see
/// `TilesetImage`'s own doc comment for what an unmatched tileset does).
/// Every other layer is ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadTilemap<'a> {
    /// Raw `.tmx` file bytes.
    pub tmx_bytes: &'a [u8],
    /// One entry per tileset the `.tmx` should actually draw from.
    pub tileset_images: Vec<TilesetImage<'a>>,
}

impl Message for LoadTilemap<'_> {
    const TOPIC: &'static str = "game-core/load-tilemap";
}

impl EncodeMessage for LoadTilemap<'_> {
    fn encode(&self) -> Vec<u8> {
        let count: u16 = self
            .tileset_images
            .len()
            .try_into()
            .expect("more than u16::MAX tileset images in one LoadTilemap");
        let mut writer = Writer::new().blob(self.tmx_bytes).u16(count);
        for image in &self.tileset_images {
            writer = writer
                .str(image.name)
                .u32(image.sprite_id)
                .blob(image.png_bytes);
        }
        writer.finish()
    }
}

impl<'a> DecodeMessage<'a> for LoadTilemap<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let tmx_bytes = reader.read_blob()?;
        let count = reader.read_u16()?;
        let mut tileset_images = Vec::with_capacity(count as usize);
        for _ in 0..count {
            tileset_images.push(TilesetImage {
                name: reader.read_str()?,
                sprite_id: reader.read_u32()?,
                png_bytes: reader.read_blob()?,
            });
        }
        reader.finish()?;
        Ok(Self {
            tmx_bytes,
            tileset_images,
        })
    }
}
