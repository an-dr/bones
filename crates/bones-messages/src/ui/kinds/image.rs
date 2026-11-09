use crate::ui::WidgetProps;
use crate::{DecodeError, Reader, Writer};

/// How an `Image` fills its box when the shapes differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// Stretched to the box.
    Fill,
    /// Scaled to cover the box, the overflow cropped evenly.
    Cover,
    /// Scaled to fit inside the box, centred.
    Contain,
}

impl Fit {
    fn tag(self) -> u8 {
        match self {
            Fit::Fill => 0,
            Fit::Cover => 1,
            Fit::Contain => 2,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            0 => Ok(Fit::Fill),
            1 => Ok(Fit::Cover),
            2 => Ok(Fit::Contain),
            _ => Err(DecodeError::InvalidTag {
                message: "ui image fit",
                tag,
            }),
        }
    }
}

/// An uploaded image (see `ui::SetImage`). Kind `"image"`; a click reports
/// `Clicked` when the id is not zero. A handle with no image yet draws an
/// empty box of the same size, so a grid does not jump as images arrive.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// The owner's handle for the image.
    pub handle: u32,
    /// Box width in points; `0` takes the image's own width.
    pub width: f32,
    /// Box height in points; `0` takes the image's own height.
    pub height: f32,
    /// How the image fills the box.
    pub fit: Fit,
    /// Corner radius in points.
    pub corner: f32,
}

impl WidgetProps for Image {
    const KIND: &'static str = "image";

    fn encode(&self) -> Vec<u8> {
        Writer::new()
            .u32(self.handle)
            .f32(self.width)
            .f32(self.height)
            .u8(self.fit.tag())
            .f32(self.corner)
            .finish()
    }

    fn decode(props: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(props);
        let image = Self {
            handle: reader.read_u32()?,
            width: reader.read_f32()?,
            height: reader.read_f32()?,
            fit: Fit::from_tag(reader.read_u8()?)?,
            corner: reader.read_f32()?,
        };
        reader.finish()?;
        Ok(image)
    }
}
