use bones_messages::EncodeMessage;
use bus::{Bus, Envelope};

pub(crate) fn publish<M: EncodeMessage>(bus: &Bus, message: M) {
    bus.publish(Envelope {
        topic: M::TOPIC.to_string(),
        sender: "ui".to_string(),
        correlation: None,
        payload: message.encode(),
    });
}

/// Splits egui's own texture-id namespace into one `u64` key: `Managed` and
/// `User` ids each start at 0, so they'd otherwise collide.
pub(crate) fn compute_texture_key(id: egui::TextureId) -> u64 {
    match id {
        egui::TextureId::Managed(n) => n << 1,
        egui::TextureId::User(n) => (n << 1) | 1,
    }
}

/// `Color32` is always premultiplied-alpha (epaint's own convention, both
/// for mesh vertex colors and image pixel data); converted to straight
/// alpha here so the renderer can use SDL's standard (non-premultiplied)
/// alpha-blend mode consistently for every ui draw call.
pub(crate) fn convert_color_image_to_straight_rgba(image: &egui::ColorImage) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        bytes.extend_from_slice(&pixel.to_srgba_unmultiplied());
    }
    bytes
}
