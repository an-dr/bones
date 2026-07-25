use bones_messages::gfx::SetCamera;
use glam::Vec2;

/// Camera-follow configuration and its eased runtime position.
pub(crate) struct Camera {
    follow_entity_id: Option<u32>,
    viewport: Vec2,
    zoom: f32,
    position: Vec2,
    responsiveness: f32,
    target_available: bool,
}

impl Camera {
    pub(crate) fn new() -> Self {
        Self {
            follow_entity_id: None,
            viewport: Vec2::ZERO,
            zoom: 1.0,
            position: Vec2::ZERO,
            responsiveness: 0.0,
            target_available: false,
        }
    }

    pub(crate) fn set_follow(
        &mut self,
        entity_id: u32,
        viewport_w: f32,
        viewport_h: f32,
        zoom: f32,
    ) {
        self.follow_entity_id = Some(entity_id);
        self.viewport = Vec2::new(viewport_w, viewport_h);
        self.zoom = zoom;
    }

    pub(crate) fn set_responsiveness(&mut self, responsiveness: f32) {
        self.responsiveness = responsiveness.max(0.0);
    }

    pub(crate) fn get_followed_entity_id(&self) -> Option<u32> {
        self.follow_entity_id
    }

    /// Advances toward `target_center`, then clamps to the loaded level.
    pub(crate) fn advance(
        &mut self,
        target_center: Option<Vec2>,
        level_size_px: Option<(f32, f32)>,
        dt: f32,
    ) {
        let Some(target_center) = target_center else {
            self.position = Vec2::ZERO;
            self.target_available = false;
            return;
        };

        self.target_available = true;
        let effective_viewport = self.viewport / self.zoom;
        let target = target_center - effective_viewport / 2.0;
        let factor = (self.responsiveness * dt).clamp(0.0, 1.0);
        self.position = if self.responsiveness == 0.0 {
            target
        } else {
            self.position + (target - self.position) * factor
        };
        if let Some((level_w, level_h)) = level_size_px {
            let maximum = (Vec2::new(level_w, level_h) - effective_viewport).max(Vec2::ZERO);
            self.position = self.position.clamp(Vec2::ZERO, maximum);
        }
    }

    pub(crate) fn build_gfx_command(&self) -> SetCamera {
        if self.follow_entity_id.is_none() || !self.target_available {
            return SetCamera {
                x: 0.0,
                y: 0.0,
                zoom: 1.0,
            };
        }
        SetCamera {
            x: self.position.x,
            y: self.position.y,
            zoom: self.zoom,
        }
    }
}
